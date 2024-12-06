# **Anno**
Anno is a **GitHub Action** that leverages LLMs to summarise code changes released between workflow runs and posts them to Slack:

<img src="docs/release_summary_example.png" alt="Release summary example" width="650">

It can also be integrated with **Jira** to fetch titles for any ticket numbers found in commit messages, however this requires your branch naming convention to include the Jira ticket number (e.g., `feature/<project-key>-1234-add-new-feature`).

## **Usage**

Place Anno as the **last job** in your workflow to ensure it runs only after all other jobs complete successfully.

The minimum required inputs to get going are `chat_gpt_api_key` and `slack_webhook_url`.

```yaml
uses: The-Sole-Supplier/anno@v1
with:
  # App name for the Slack message.
  # Default: Repository name.
  app_name: ""

  # ChatGPT API key for chat completions.
  # Required.
  chat_gpt_api_key: ""

  # ChatGPT model to use.
  # Default: `gpt-4o`.
  chat_gpt_model: ""

  # Enable Jira integration.
  # Default: `false`.
  jira_integration_enabled: "false"

  # Jira username and API key (base64 encoded `<username>:<api_token>`).
  # Required if Jira is enabled.
  jira_api_key: ""

  # Jira instance base URL (e.g., https://my-company.atlassian.net).
  # Required if Jira is enabled.
  jira_base_url: ""

  # Jira project key.
  # Required if Jira is enabled.
  jira_project_key:

  # Slack webhook URL for the release summary.
  # Required.
  slack_webhook_url: ""
```

## Monorepo Usage

There shouldn't be any special setup required for monorepos. Anno will download the workflow file and use the [`on.push.paths`](https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions#example-including-paths) and [`on.push.paths-ignore`](https://docs.github.com/en/actions/writing-workflows/workflow-syntax-for-github-actions#example-excluding-paths) properties to determine which files and commits to include in its analysis:

```yaml
on:
  push:
    paths:
      - 'sub-project/**'
      - '!sub-project/docs/**'
```

If `paths` is not specified, Anno will default to the entire repository.

## API Alternative

Anno can also be deployed as an AWS HTTP Lambda that integrates with Jira and GitHub webhook events. In addition to summarising releases, it can review pull requests and add test cases to Jira issues.

For more details, see the API's [README](api/README.md).