# C#

C# syntax, folding, source structure and editing rules, with code intelligence
through `csharp-ls` on the selected project execution host. The public language
ID remains `csharp`; `.cs` files retain the same independently installed parser.

This API 8 component uses the public native-server contract in local and remote
projects. Updating from syntax-only releases requires the ordinary review for
native process execution. Disablement or missing tooling leaves editing and
installed syntax support available.

## Host tools and projects

Install the .NET 10 SDK and `csharp-ls` 0.28.0 on the execution host, with both
`dotnet` and `csharp-ls` on its PATH. Open WebIDE execution-host images include
these tools. For a paired host, use the normal .NET tool installation:

```sh
dotnet tool install --global csharp-ls --version 0.28.0
# If already installed:
dotnet tool update --global csharp-ls --version 0.28.0
```

The host also needs the SDKs and restored dependencies required by your project.
The server discovers `.slnx`, existing `.sln` and `.csproj` files from the project
root. New solutions should use `.slnx`. Its workspace configuration is under
`csharp`; project-specific compiler and formatting settings remain in project
files and `.editorconfig`. The package enables `csharp.useMetadataUris` and declares
the `csharp` URI scheme through the public API 8 `virtualDocuments` contract.
Framework and generated definitions are retrieved from the same server through
`csharp/metadata` and shown read-only. This requires an API 8 host; older hosts
reject the package. Project definitions use ordinary file URIs.

The shared Code intelligence controls provide diagnostics, completion, symbol
information, definitions, references, symbols, formatting, rename and supported
text-edit code actions. Formatting and multi-file edits are reviewed before
staging in editor tabs; Save writes the staged changes. Installation, provider
preferences, updates, disablement and removal use the ordinary plugin lifecycle.
