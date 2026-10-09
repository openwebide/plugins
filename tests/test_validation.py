import copy
import json
import subprocess
import sys
import tempfile
import unittest
from pathlib import Path

ROOT = Path(__file__).resolve().parents[1]
sys.path.insert(0, str(ROOT / "scripts"))
import validate


class ValidationTests(unittest.TestCase):
    @classmethod
    def setUpClass(cls):
        cls.checks = validate.validators()

    def setUp(self):
        self.temp = tempfile.TemporaryDirectory()
        self.addCleanup(self.temp.cleanup)
        self.root = Path(self.temp.name)
        self.package = self.root / "plugins/pr-review"
        self.skill = self.package / "skills/review-changes/SKILL.md"
        self.skill.parent.mkdir(parents=True)
        self.skill.write_text("---\nname: review-changes\ndescription: Review changes.\n---\n\nRead the diff.\n")
        self.manifest = json.loads((ROOT / "plugins/pr-review/plugin.json").read_text())
        self.manifest.pop("readme")
        self.save_manifest()
        self.catalog = {"schemaVersion": 1, "name": "Test", "plugins": []}

    def save_manifest(self):
        (self.package / "plugin.json").write_text(json.dumps(self.manifest))

    def listing(self, **source):
        return {
            "publisher": self.manifest["publisher"], "name": self.manifest["name"],
            "displayName": self.manifest["displayName"], "description": self.manifest["description"],
            "categories": ["skills"],
            "releases": [{"version": "0.1.0", "source": {
                "commit": "a" * 40, "path": "plugins/pr-review", **source,
            }}],
        }

    def test_reference_repository_and_standalone_package(self):
        self.assertEqual(validate.validate_repository(ROOT), (5, 1))
        self.assertEqual(validate.validate_package(self.package, self.checks)["name"], "pr-review")

    def test_unknown_contributions_and_api_are_rejected(self):
        for field in ("mcpServers", "languages", "panels", "hooks"):
            with self.subTest(field=field):
                value = copy.deepcopy(self.manifest)
                value["contributions"][field] = []
                with self.assertRaises(validate.Invalid):
                    validate.check_schema(value, self.checks["Plugin"], "plugin")
        self.manifest["compatibility"]["pluginApi"] = 3
        self.save_manifest()
        with self.assertRaises(validate.Invalid):
            validate.validate_package(self.package, self.checks)

    def test_platform_tool_groups_require_api_two_and_have_bounded_known_capabilities(self):
        value = copy.deepcopy(self.manifest)
        value["contributions"]["skills"] = []
        value["contributions"]["toolGroups"] = ["web"]
        with self.assertRaises(validate.Invalid):
            validate.check_schema(value, self.checks["Plugin"], "plugin")
        value["compatibility"]["pluginApi"] = 2
        validate.check_schema(value, self.checks["Plugin"], "plugin")
        for groups in (["web", "web"], ["arbitrary-execution"], []):
            value["contributions"]["toolGroups"] = groups
            with self.assertRaises(validate.Invalid):
                validate.check_schema(value, self.checks["Plugin"], "plugin")

    def test_empty_and_duplicate_skills(self):
        self.manifest["contributions"]["skills"] = []
        self.save_manifest()
        with self.assertRaises(validate.Invalid):
            validate.validate_package(self.package, self.checks)
        self.manifest["contributions"]["skills"] = [{"path": "skills/review-changes/SKILL.md"}] * 2
        self.save_manifest()
        with self.assertRaises(validate.Invalid):
            validate.validate_package(self.package, self.checks)

    def test_invalid_versions_and_identity(self):
        for version in ("latest", "01.0.0", "1.0", "1.0.0-01", "1.0.0\n"):
            with self.subTest(version=version):
                value = copy.deepcopy(self.manifest)
                value["version"] = version
                with self.assertRaises(validate.Invalid):
                    validate.check_schema(value, self.checks["Plugin"], "plugin")
        self.manifest["name"] = "PR Review"
        self.save_manifest()
        with self.assertRaises(validate.Invalid):
            validate.validate_package(self.package, self.checks)

    def test_semver_prerelease_and_build_metadata(self):
        self.manifest["version"] = "1.2.3-beta.1+build.42"
        self.save_manifest()
        validate.validate_package(self.package, self.checks)

    def test_missing_and_traversal_paths(self):
        for path in ("../outside/SKILL.md", "/tmp/SKILL.md", "skills/../SKILL.md",
                     "skills\\SKILL.md", "skills/missing/SKILL.md"):
            with self.subTest(path=path):
                self.manifest["contributions"]["skills"] = [{"path": path}]
                self.save_manifest()
                with self.assertRaises(validate.Invalid):
                    validate.validate_package(self.package, self.checks)

    def test_symlinked_skill_and_resource(self):
        external = self.root / "outside.md"
        external.write_text("outside")
        link = self.skill.parent / "linked.md"
        link.symlink_to(external)
        with self.assertRaisesRegex(validate.Invalid, "symlinks"):
            validate.validate_package(self.package, self.checks)
        link.unlink()
        self.skill.unlink()
        self.skill.symlink_to(external)
        with self.assertRaisesRegex(validate.Invalid, "symlinks"):
            validate.validate_package(self.package, self.checks)

    def test_invalid_and_duplicate_frontmatter(self):
        for text in (
            "No frontmatter", "---\nname: review-changes\n", "---\n- item\n---\nBody",
            "---\nname: review-changes\nname: review-changes\ndescription: Review\n---\nBody",
            "---\nname: wrong-name\ndescription: Review\n---\nBody",
            "---\nname: review-changes\ndescription: ' '\n---\nBody",
            "---\nname: review-changes\ndescription: Review\n---\n",
        ):
            with self.subTest(text=text):
                self.skill.write_text(text)
                with self.assertRaises(validate.Invalid):
                    validate.validate_package(self.package, self.checks)

    def test_duplicate_json_keys_and_nonfinite_values(self):
        path = self.package / "plugin.json"
        for text in ('{"name":"one","name":"two"}', '{"value":NaN}'):
            path.write_text(text)
            with self.assertRaises(validate.Invalid):
                validate.read_json(path)

    def test_skill_resource_limits(self):
        for index in range(17):
            (self.skill.parent / f"resource-{index}.md").write_text("content")
        with self.assertRaisesRegex(validate.Invalid, "16 resources"):
            validate.validate_package(self.package, self.checks)
        for path in self.skill.parent.glob("resource-*.md"):
            path.unlink()
        (self.skill.parent / "big.md").write_text("a" * 32769)
        with self.assertRaisesRegex(validate.Invalid, "32 KiB"):
            validate.validate_package(self.package, self.checks)

    def test_binary_resource_encoding_limit(self):
        resource = self.skill.parent / "asset.bin"
        resource.write_bytes(b"\xff" * 24576)
        validate.validate_package(self.package, self.checks)
        resource.write_bytes(b"\xff" * 24577)
        with self.assertRaisesRegex(validate.Invalid, "base64"):
            validate.validate_package(self.package, self.checks)

    def test_resource_names_and_nested_manifests_match_importer(self):
        for name in ("unsafe@.md", "nested/SKILL.md"):
            with self.subTest(name=name):
                path = self.skill.parent / name
                path.parent.mkdir(parents=True, exist_ok=True)
                path.write_text("content")
                with self.assertRaises(validate.Invalid):
                    validate.validate_package(self.package, self.checks)
                path.unlink()

    def test_duplicate_skill_names_across_directories(self):
        second = self.package / "other/review-changes/SKILL.md"
        second.parent.mkdir(parents=True)
        second.write_text(self.skill.read_text())
        self.manifest["contributions"]["skills"].append({"path": "other/review-changes/SKILL.md"})
        self.save_manifest()
        with self.assertRaisesRegex(validate.Invalid, "duplicate contributed skill name"):
            validate.validate_package(self.package, self.checks)

    def test_commit_formats_and_root_package(self):
        self.catalog["plugins"] = [self.listing(path=".")]
        validate.validate_catalog(self.catalog, self.checks)
        self.catalog["plugins"] = [self.listing(commit="b" * 64)]
        validate.validate_catalog(self.catalog, self.checks)

    def test_catalog_rejects_mutable_release_refs_and_traversal(self):
        for source in ({"commit": "main"}, {"commit": "v0.1.0"}, {"path": "../plugin"}, {"path": "/plugin"}):
            with self.subTest(source=source):
                self.catalog["plugins"] = [self.listing(**source)]
                with self.assertRaises(validate.Invalid):
                    validate.validate_catalog(self.catalog, self.checks)

    def test_catalog_cannot_redirect_packages_to_another_repository(self):
        for location in ("listing", "release", "source"):
            with self.subTest(location=location):
                listing = self.listing()
                target = {"listing": listing, "release": listing["releases"][0],
                          "source": listing["releases"][0]["source"]}[location]
                target["repository"] = "https://git.example.org/other/plugins.git"
                self.catalog["plugins"] = [listing]
                with self.assertRaises(validate.Invalid):
                    validate.validate_catalog(self.catalog, self.checks)

    def test_duplicate_catalog_identities_and_versions(self):
        listing = self.listing()
        self.catalog["plugins"] = [listing, copy.deepcopy(listing)]
        with self.assertRaisesRegex(validate.Invalid, "duplicate catalog identity"):
            validate.validate_catalog(self.catalog, self.checks)
        listing["releases"].append(copy.deepcopy(listing["releases"][0]))
        self.catalog["plugins"] = [listing]
        with self.assertRaisesRegex(validate.Invalid, "duplicate release version"):
            validate.validate_catalog(self.catalog, self.checks)

    def test_local_catalog_metadata_matches_manifest(self):
        listing = self.listing()
        listing["releases"][0]["version"] = self.manifest["version"]
        listing["description"] = "Incorrect description"
        self.catalog["plugins"] = [listing]
        (self.root / "marketplace.json").write_text(json.dumps(self.catalog))
        with self.assertRaisesRegex(validate.Invalid, "description differs"):
            validate.validate_repository(self.root)

    def test_cli_failure_has_actionable_error_and_nonzero_exit(self):
        self.skill.unlink()
        result = subprocess.run(
            [sys.executable, str(ROOT / "scripts/validate.py"), "--package", str(self.package)],
            capture_output=True, text=True, check=False,
        )
        self.assertEqual(result.returncode, 1)
        self.assertIn("missing package file", result.stderr)
        self.assertNotIn("Traceback", result.stderr)


if __name__ == "__main__":
    unittest.main()
