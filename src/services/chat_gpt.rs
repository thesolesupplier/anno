use chatgpt::prelude::*;
use std::env;

pub async fn get_diff_summary(diff: &str) -> Result<String> {
    let openai_api_key = env::var("OPENAI_API_KEY").expect("OPENAI_API_KEY should be set");

    let chat_gpt = ChatGPT::new_with_config(
        openai_api_key,
        ModelConfigurationBuilder::default()
            .temperature(0.2)
            .top_p(0.1)
            .frequency_penalty(2.0)
            .build()
            .unwrap(),
    )?;

    let message = format!("
        Prompt:
            I need you to analyze a git code diff to identify and summarize the user-facing features that have been released.
            Avoid describing each individual code change. Instead, focus on understanding the broader context of the changes and how they translate into new features or improvements for end-users.
            Additionally, list any dependency changes that were made and specify any feature flags connected to these features, along with the environments they are enabled in.
            Additionally, DO NOT list the feature flags under their own separate heading. Instead, include them in the relevant feature descriptions.
            Please ensure that you DO NOT include headings in your summary if there are no changes related to them.
        Steps:
            Analyze the Diff: Examine the git code diff to understand the changes in the codebase.
            Identify User-Facing Features: Determine which changes correspond to new features, enhancements, or bug fixes that would be noticeable to the end-users.
            Summarize in Non-Technical Terms: Write a summary of these features in a way that a non-technical team can understand, focusing on the benefits or changes that users will experience.
            List Dependency Changes: Identify any changes in the project's dependencies (e.g., new libraries, updated versions) and list them.
            Identify Feature Flags: Determine which features are controlled by feature flags and specify the environments (e.g., development, staging, production) in which these flags are enabled.
            Exclude Unchanged Sections: Only include headings for New Features, Improvements, Bug Fixes, and Dependency Changes if there are relevant updates.
        Example Output:
            *New Features*:
                Enhanced Search Functionality: Users can now filter search results by date and relevance.
                    Feature Flag: `enhanced_search`
                    Enabled in: QA, Staging
                Profile Customization: Users have new options to customize their profiles, including uploading a profile picture and adding a bio.
                    Feature Flag: `profile_customization`
                    Enabled in: QA, Staging, Prod
            *Improvements*:
                Faster Load Times: Various optimizations have been made to improve the overall speed of the application.
                Improved Mobile Experience: The mobile version of the site has been revamped.
            *Bug Fixes*:
                Fixed Login Issue: Resolved an issue where some users were unable to log in due to a server error.
                Corrected Display Errors: Addressed several minor display errors on the dashboard.
            *Dependency Changes*:
                Updated Library `XYZ` to version 1.3.0.
                Added library `ABC` version 2.1.0.
        Please perform this analysis on the provided git code diff and deliver a summary as described above.
        ----
        Diff: {diff}
    ");

    let response = chat_gpt
        .send_message(message)
        .await?
        .message()
        .content
        .clone();

    Ok(response)
}
