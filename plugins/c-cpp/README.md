# C and C++

C/C++ 0.2.0 preserves syntax, folding, source structure and editing rules, and
adds clangd through the public API 7 language-server contract. The stable `c`
and `cpp` language IDs share one native service for each project/client.
Ordinary editing remains available without clangd.

## Execution host

Install clangd on the project execution host and make `clangd` available on its
PATH: the paired bridge host for local projects, or the server/container for
remote projects. Current Open WebIDE images include pinned clangd and the C/C++
build tools; native bridge archives need a separately installed compiler,
clangd and the project's headers/libraries. The browser runs no plugin code.

Enabling native language-server access requires the ordinary plugin capability
review. The syntax component still uses SDK 0.1.0 without WASM host capabilities;
native process access is a separate reviewed contribution. Older API 5/6 hosts
do not provide this package's API 7 integration.

## Project configuration

Use the project's [`compile_commands.json`](https://clang.llvm.org/docs/JSONCompilationDatabase.html) for include directories, macros,
compiler language standards and target flags. For CMake, generate it with
`-DCMAKE_EXPORT_COMPILE_COMMANDS=ON`. clangd discovers databases above source
files and in `build/`; configure another location through the project's [`.clangd`](https://clangd.llvm.org/config)
`CompileFlags.CompilationDatabase`. Simple projects can use `compile_flags.txt`
or `.clangd` `CompileFlags.Add` without a database. Keep C and C++ standards
appropriate to each file. Headers borrow a compile command from a source file;
use `.clangd` path conditions when an ambiguous header needs explicit flags.

clangd runs on demand with background indexing, unsaved header overlays and
in-memory precompiled headers. Project configuration and toolchain availability determine analysis;
Open WebIDE does not invent build flags or broadly enable `--query-driver`.
Cross-compilation system headers must be available on the execution host.

The shared editor service synchronizes unsaved buffers and supplies diagnostics,
manual hover/completion, definitions, references, document symbols, reviewed
rename, document formatting and supported text-edit code actions. Review edits
before staging them into editor tabs; Save stays explicit. Server commands and
file creation/movement/deletion remain unsupported and are rejected explicitly.
External definitions are read-only. clangd's background index can create derived
`.cache/clangd` data; analysis does not save editor buffers or change source files.

Installation, updates, provider selection, disablement and removal use the same
public plugin lifecycle as community packages. Missing or disabled providers
leave ordinary editing available. Related C/C++ dialects stay in one replaceable
package according to clangd's coverage; no built-in language fallback or
identity-specific host dispatch is used.
