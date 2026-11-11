# Go

Go syntax, folding, source structure and editing rules, with code intelligence
through `gopls serve` on the selected project execution host. The existing `go`
language ID and `.go` files retain their syntax and editing behavior.

Install Go 1.27.3 and gopls v0.24.0 on paired hosts:

```sh
go install golang.org/x/tools/gopls@v0.24.0
```

Both `go` and `gopls` must be on the bridge's `PATH`; a normal Go installation
puts gopls in `$(go env GOPATH)/bin`. OpenWebIDE's Docker image includes the
verified SDK and server. Dependencies, the Go build/module caches and any private
module credentials belong to the execution host, rather than the browser.

Open a project containing `go.mod`, or use a `go.work` file to work across modules:

```go
go 1.27.0

use (
    ./app
    ./lib
)
```

gopls owns module, workspace and build-context discovery. One project service
synchronizes the client's open buffers across packages and modules, including
unsaved changes. Open the folder containing `go.work` to edit its modules together.
Local projects use the paired native folder; remote projects use the
server's project folder. Go environment and dependency resolution follow that host.

Version 0.3.0 requires plugin API 9. Its public cache declaration redirects
`GOPLSCACHE`, `GOCACHE` and `GOMODCACHE` to native derived-data directories outside
the project. The namespace includes the reported gopls version and declared
module/workspace inputs. Go owns dependency/build freshness and private module
credentials remain on the execution host.

The independent component uses the public SDK, ordinary reviewed native
execution and shared service lifecycle. Both project modes support diagnostics,
symbol information, completion with imports, definitions, references, document
symbols, reviewed rename and document formatting. Selected completion edits enter
the active buffer; reviewed rename and formatting edits are staged in editor
history and recovery until Save. Standard-library definitions remain read-only.
Server commands and file creation, movement or deletion are not run automatically.

Missing tools, denied approval, disabled packages or server failures leave ordinary
editing available. Installation, updates, provider preferences, disablement and
removal use the normal plugin lifecycle. No Go-specific host privileges or browser
code are required.
