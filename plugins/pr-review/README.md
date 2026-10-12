# PR review

Version 0.2.0 combines the `review-changes` skill with a Rust-owned GitHub
pull-request panel through the public SDK. It declares only HTTP and private
records capabilities, with no built-in feature proxies or special privileges.
The skill supports working-tree and branch review with existing agent tools.
The panel requires an API 4 host; historical 0.1.x releases remain skill-only.

## Pull-request panel

Choose **Pull requests** in workspace navigation, enter an API URL and
`owner/repository`, then save the configuration. Public repositories can be viewed
without a token. Publishing needs a token with the selected repository's
[Pull requests write permission](https://docs.github.com/en/rest/pulls/reviews#create-a-review-for-a-pull-request).
The password field starts empty; leaving it blank preserves the saved token.
Configuration lives in the plugin's account/project-scoped records.

Open a pull request to read its description, diff excerpt and review excerpts.
Choose Comment, Approve or Request changes and enter a message. Publishing requires
app approval showing the destination, message, review type and reviewed commit.
Configuration or commit changes require a fresh view. The plugin passes the
reviewed commit explicitly to the [GitHub review API](https://docs.github.com/en/rest/pulls/reviews#create-a-review-for-a-pull-request).
SDK views only read; publishing occurs in the declared action handler.

The API URL defaults to GitHub and can point to a compatible enterprise API base.
Requests pin the supported `2022-11-28` [API version](https://docs.github.com/en/rest/about-the-rest-api/api-versions).
See [panel authoring](../../docs/panels.md) and [the Rust source](src/lib.rs).

## Try the skill today

In Open WebIDE, open **Sessions → Skills → Import folder** and select
`skills/review-changes`. Inspect the draft, save it and enable it for the project.
Ask the agent to review changes. The skill and its resource use the existing
project-skill format and importer in either workspace mode.

Manual skill import creates independent project data. Installing the plugin from
Plugins enables its review skill in existing and future projects by default.
Disable it for a project to save an opt-out. Notify is the default update policy;
select another release or use Update All to change the installed version.
Running tasks retain their original instructions/resources.
