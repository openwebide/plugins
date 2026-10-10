"""Offline package/catalog validation. Never executes package code or fetches Git."""

from __future__ import annotations

import argparse
import json
import re
import sys
from pathlib import Path, PurePosixPath

import yaml
from jsonschema import Draft202012Validator
from referencing import Registry, Resource

SCHEMAS = Path(__file__).resolve().parents[1] / "schemas"
NAME = re.compile(r"[a-z0-9]+(?:-[a-z0-9]+)*\Z")


class Invalid(ValueError):
    """Invalid repository or package content."""


def unique_pairs(pairs):
    result = {}
    for key, value in pairs:
        if key in result:
            raise Invalid(f"duplicate key: {key}")
        result[key] = value
    return result


class UniqueLoader(yaml.SafeLoader):
    pass


def yaml_mapping(loader, node):
    loader.flatten_mapping(node)
    pairs = [
        (loader.construct_object(key), loader.construct_object(value))
        for key, value in node.value
    ]
    if any(not isinstance(key, str) for key, _ in pairs):
        raise Invalid("YAML mapping keys must be strings")
    return unique_pairs(pairs)


UniqueLoader.add_constructor("tag:yaml.org,2002:map", yaml_mapping)


def read_json(path):
    def invalid_constant(value):
        raise Invalid(f"invalid JSON constant: {value}")

    return json.loads(
        path.read_text(encoding="utf-8"),
        object_pairs_hook=unique_pairs,
        parse_constant=invalid_constant,
    )


def validators():
    schemas = [read_json(path) for path in sorted(SCHEMAS.glob("*.schema.json"))]
    registry = Registry().with_resources([
        (schema["$id"], Resource.from_contents(schema)) for schema in schemas
    ])
    result = {}
    for schema in schemas:
        Draft202012Validator.check_schema(schema)
        result[schema["title"]] = Draft202012Validator(schema, registry=registry)
    return result


def check_schema(value, validator, label):
    errors = sorted(validator.iter_errors(value), key=lambda error: str(list(error.path)))
    if errors:
        details = [f"{label}/{ '/'.join(map(str, e.path)) }: {e.message}" for e in errors]
        raise Invalid("\n".join(details))


def package_path(root, relative):
    """Accept normalized POSIX paths inside a package, without links."""
    parts = PurePosixPath(relative).parts
    if (not relative or "\\" in relative or "\x00" in relative
            or relative.startswith("/") or not parts
            or any(part in ("", ".", "..") for part in relative.split("/"))):
        raise Invalid(f"invalid package path: {relative!r}")
    current = root
    for part in parts:
        current = current / part
        if current.is_symlink():
            raise Invalid(f"package paths cannot contain symlinks: {relative}")
    if not current.is_file():
        raise Invalid(f"missing package file: {relative}")
    return current


def validate_skill(root, relative):
    skill = package_path(root, relative)
    if skill.name != "SKILL.md":
        raise Invalid(f"skill entry must point to SKILL.md: {relative}")
    if skill.stat().st_size > 131072:
        raise Invalid(f"skill exceeds 128 KiB: {relative}")
    text = skill.read_text(encoding="utf-8")
    lines = text.splitlines()
    if not lines or lines[0] != "---":
        raise Invalid(f"missing YAML frontmatter: {relative}")
    try:
        end = lines.index("---", 1)
    except ValueError as error:
        raise Invalid(f"unterminated YAML frontmatter: {relative}") from error
    metadata = yaml.load("\n".join(lines[1:end]), Loader=UniqueLoader)
    if not isinstance(metadata, dict):
        raise Invalid(f"frontmatter must be a mapping: {relative}")
    name, description = metadata.get("name"), metadata.get("description")
    if not isinstance(name, str) or len(name) > 64 or not NAME.fullmatch(name):
        raise Invalid(f"invalid skill name: {relative}")
    if skill.parent.name != name:
        raise Invalid(f"skill directory must match name {name!r}: {relative}")
    if (not isinstance(description, str) or not description.strip()
            or len(description) > 1024):
        raise Invalid(f"invalid skill description: {relative}")
    instructions = "\n".join(lines[end + 1:]).strip()
    if not instructions or len(instructions) > 32768:
        raise Invalid(f"instructions must contain 1–32768 characters: {relative}")
    extra = {key: value for key, value in metadata.items()
             if key not in ("name", "description")}
    try:
        size = len(json.dumps(extra, ensure_ascii=False, allow_nan=False).encode())
    except (TypeError, ValueError, RecursionError) as error:
        raise Invalid(f"skill metadata must be JSON-compatible: {relative}") from error
    size += len(name.encode()) + len(description.encode()) + len(instructions.encode())
    import_size = len(skill.name.encode()) + skill.stat().st_size
    resources = []
    for path in sorted(skill.parent.rglob("*")):
        if path.is_symlink():
            raise Invalid(f"skill resources cannot contain symlinks: {path}")
        if path.is_dir() or path == skill:
            continue
        if not path.is_file():
            raise Invalid(f"unsupported skill resource: {path}")
        resource_name = path.relative_to(skill.parent).as_posix()
        package_path(root, path.relative_to(root).as_posix())
        if path.name == "SKILL.md":
            raise Invalid(f"skill folder must contain exactly one SKILL.md: {relative}")
        if not re.fullmatch(r"[A-Za-z0-9_. /-]+", resource_name):
            raise Invalid(f"invalid skill resource name: {resource_name}")
        if len(resource_name.encode()) > 160:
            raise Invalid(f"resource name exceeds 160 bytes: {resource_name}")
        if path.stat().st_size > 32768:
            raise Invalid(f"resource exceeds 32 KiB: {resource_name}")
        raw = path.read_bytes()
        import_size += len(resource_name.encode()) + len(raw)
        try:
            raw.decode("utf-8")
            content_size = len(raw)
        except UnicodeDecodeError:
            content_size = 4 * ((len(raw) + 2) // 3)
            if content_size > 32768:
                raise Invalid(f"base64 resource exceeds 32 KiB: {resource_name}")
        size += len(resource_name.encode()) + content_size
        resources.append(resource_name)
    if len(resources) > 16 or size > 131072 or import_size > 131072:
        raise Invalid(f"skill exceeds 16 resources or 128 KiB: {relative}")
    return name


def validate_package(root, checks=None):
    root = Path(root)
    if root.is_symlink() or not root.is_dir():
        raise Invalid(f"package must be a real directory: {root}")
    manifest_path = package_path(root, "plugin.json")
    manifest = read_json(manifest_path)
    checks = checks or validators()
    check_schema(manifest, checks["Plugin"], str(manifest_path))
    names = set()
    for contribution in manifest["contributions"]["skills"]:
        name = validate_skill(root, contribution["path"])
        if name in names:
            raise Invalid(f"duplicate contributed skill name: {name}")
        names.add(name)
    if "executable" in manifest:
        for path in ("Cargo.toml", "Cargo.lock"):
            package_path(root, path)
        tools = manifest["contributions"]["tools"]
        if len({tool["name"] for tool in tools}) != len(tools):
            raise Invalid("duplicate executable tool name")
    if "readme" in manifest:
        package_path(root, manifest["readme"])
    return manifest


def validate_catalog(catalog, checks=None):
    checks = checks or validators()
    check_schema(catalog, checks["Marketplace"], "marketplace")
    identities = set()
    for plugin in catalog["plugins"]:
        identity = (plugin["publisher"], plugin["name"])
        if identity in identities:
            raise Invalid(f"duplicate catalog identity: {'/'.join(identity)}")
        identities.add(identity)
        versions = set()
        for release in plugin["releases"]:
            if release["version"] in versions:
                raise Invalid(f"duplicate release version: {release['version']}")
            versions.add(release["version"])
    return catalog


def validate_repository(root):
    root = Path(root)
    checks = validators()
    catalog = validate_catalog(read_json(root / "marketplace.json"), checks)
    packages = {}
    directory = root / "plugins"
    if directory.is_symlink():
        raise Invalid("plugins directory cannot be a symlink")
    for path in sorted(directory.iterdir()) if directory.exists() else []:
        if path.is_symlink():
            raise Invalid(f"package directory cannot be a symlink: {path}")
        if not path.is_dir():
            continue
        manifest = validate_package(path, checks)
        identity = (manifest["publisher"], manifest["name"])
        if identity in packages:
            raise Invalid(f"duplicate package identity: {'/'.join(identity)}")
        packages[identity] = manifest
    for listing in catalog["plugins"]:
        manifest = packages.get((listing["publisher"], listing["name"]))
        if manifest is None:
            continue  # Historical packages may be absent from the current checkout.
        if any(release["version"] == manifest["version"] for release in listing["releases"]):
            for key in ("displayName", "description"):
                if listing[key] != manifest[key]:
                    raise Invalid(f"catalog {key} differs from local manifest: {manifest['name']}")
            categories = []
            if manifest["contributions"]["skills"]:
                categories.append("skills")
            if manifest["contributions"].get("toolGroups") or manifest["contributions"].get("tools"):
                categories.append("tools")
            if sorted(listing["categories"]) != categories:
                raise Invalid(f"catalog categories differ from local contributions: {manifest['name']}")
    return len(packages), len(catalog["plugins"])


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("--root", type=Path, default=Path(__file__).resolve().parents[1])
    parser.add_argument("--package", type=Path, help="Validate a standalone package directory")
    args = parser.parse_args()
    try:
        if args.package:
            manifest = validate_package(args.package)
            print(f"Valid package: {manifest['publisher']}/{manifest['name']} {manifest['version']}")
        else:
            count, listings = validate_repository(args.root)
            print(f"Valid: {count} packages, {listings} catalog listings")
    except (Invalid, OSError, UnicodeError, ValueError, yaml.YAMLError) as error:
        print(f"Validation failed: {error}", file=sys.stderr)
        return 1
    return 0


if __name__ == "__main__":
    sys.exit(main())
