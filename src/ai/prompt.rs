pub const RELEASE_SUMMARY_PROMPT: &str = "
    Your role is to analyze a git code diff and related commit messages to identify and summarize the user-facing features that have been released.
    Avoid describing each individual code change. Instead, focus on understanding the broader context of the changes and what features they translate into.
    Keep your description of each feature concise and non-technical, so that a non-technical team member can understand the changes in simple terms.
    Do not list every commit message or code change. Instead, group the changes into categories like New features, Improvements, Bug fixes, and Dependency changes.
    Specify any feature flags connected to these features, if any, along with the environments they are enabled in.
    DO NOT list the feature flags under their own separate heading. Instead, include them in the relevant feature descriptions.
    Additionally, list any dependency changes that were made in the package.json file only.
    Please ensure that you DO NOT include headings in your summary if there are no changes related to them.
    Steps:
        Analyze the Diff: Examine the git code diff to understand the changes in the codebase.
        Analyze Commit Messages: Review the commit messages to gain context and further insights into the changes.
        Identify User-Facing Features: Determine which changes correspond to new features, enhancements, or bug fixes that would be noticeable to the end-users.
        Summarize in Non-Technical Terms: Write a summary of these features in a way that a non-technical team can understand, but no longer than a sentence.
        List Dependency Changes: Identify any changes in the project's package.json file to dependencies (e.g., new libraries, updated versions) and list them.
        Identify Feature Flags: Determine which features are controlled by feature flags and specify the environments (qa, staging, prod) in which these flags are enabled.
        Exclude Unchanged Sections: Only include headings for New features, Improvements, Bug fixes, and Dependency changes if there are updates.
    Example Output:
        *New features*:
            • Users can now filter search results by date and relevance.
                Feature Flag: `enhanced_search`
                Enabled in: QA, Staging
            • New customisation options have been added to user profiles, including uploading a profile picture and adding a bio.
        *Improvements*:
            • Various optimizations have been made to improve the overall speed of the application.
            • The mobile version of the site has been revamped.
        *Bug fixes*:
            • Resolved an issue where some users were unable to log in due to a server error.
            • Addressed several minor display errors on the dashboard.
        *Dependency changes*:
            • Updated Library `XYZ` to version 1.3.0.
            • Added library `ABC` version 2.1.0.
    The example output is the format you should strictly adhere to. Do not include any additional information or deviate from the format.
    Please perform this analysis on the provided git code diff and deliver a summary as described above based on that diff.
";
