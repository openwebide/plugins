# Terraform

One independent package for the Terraform dialects served by HashiCorp's
`terraform-ls`: Terraform (`.tf`), variables (`.tfvars`), stacks
(`.tfcomponent.hcl`), deployments (`.tfdeploy.hcl`), searches (`.tfquery.hcl`),
policies (`.policy.hcl`) and policy tests (`.policytest.hcl`). Terraform JSON and
unrelated HCL, including Packer, are not sent to this server.

The package owns its HCL grammar bindings, colors, protected string/comment
regions, folding and editing rules. It uses the public parser toolkit and SDK;
adding this language requires no app-owned language ID or built-in parser switch.
The component executes on the project's native host. Browser clients render
validated analysis and send unsaved source through the shared language service.

Version 0.1.0 requires plugin API 9 for declared project configuration/dependency
inputs. Install Terraform 1.17.1 and terraform-ls 0.39.1 on the execution host and
put both binaries on its `PATH`. The Open WebIDE tool installer pins official
HashiCorp distributions by checksum. Open WebIDE maintains this packaging;
HashiCorp maintains the upstream CLI and language server. The HCL grammar is
`tree-sitter-hcl` 1.1.0 under Apache-2.0.

The server's advertised capabilities determine available code intelligence.
Formatting requires the Terraform CLI. Provider-specific information requires
the project's normal initialized dependencies; this package does not initialize,
plan or apply infrastructure automatically. Review any proposed server command
through the ordinary native command approval flow. Installation, provider
selection, disablement, removal and updates follow the public lifecycle in both
project modes. Missing tools or failed analysis leave ordinary editing available.

The package pins its public parser toolkit dependencies to immutable Git commits.
While developing both repositories, validate against a local app checkout:

```sh
python3 scripts/validate_rust.py --sdk ../openwebide/crates/plugin-sdk \
  --language-toolkit ../openwebide --package plugins/terraform
```

The development check uses a disposable package copy and leaves the source
release lock and Git pins unchanged. After changing a release pin, regenerate
the package lockfile and validate without `--language-toolkit` to check the
published dependency rather than the local override.
