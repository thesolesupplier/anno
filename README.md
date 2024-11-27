# **Anno**
Anno is a **GitHub Action** that leverages LLMs to summarise code changes released in deployments and posts them to Slack:

<img src="docs/release_summary_example.png" alt="Release summary example" width="650">

It can also be integrated with **Jira** to fetch issue titles for any ticket numbers referenced in commits.

## **Usage**

```yaml
uses: The-Sole-Supplier/anno
with:
  # App name for the Slack message. Defaults to the repository name.
  app_name: ""

  # ChatGPT API key for chat completions. Required.
  chat_gpt_api_key: ""

  # ChatGPT model to use. Defaults to `latest`.
  chat_gpt_model: ""

  # Enable Jira integration. Defaults to `false`.
  jira_integration_enabled: "false"

  # Jira username and API key (base64 encoded `<username>:<api_token>`). Required if Jira is enabled.
  jira_api_key: ""

  # Jira instance base URL (e.g., https://my-company.atlassian.net). Required if Jira is enabled.
  jira_base_url: ""

  # Jira project key. Required if Jira is enabled.
  jira_project_key:

  # Slack webhook URL for the release summary. Required.
  slack_webhook_url: ""
```

## Local Development and Deployment

For details on how to run and deploy the action locally, please refer to the [Local Development](action/README.md#local-development) and [Deployment](action/README.md#deployment) sections in the action's [README](action/README.md).
## Alternative Webhook Integration

Anno can also be deployed as an **AWS HTTP Lambda** that receives Jira and GitHub webhook events to, in addition to summarising releases, review PRs and add test cases to Jira issues.

For more details, please refer to the API's [README](api/README.md).