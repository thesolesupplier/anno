pub const RELEASE_SUMMARY_PROMPT: &str = "
    <Instructions>
        Your role is to analyze a git code diff and related commit messages to identify and summarize the features that have been released.
        Avoid describing each individual code change. Instead, focus on understanding the broader context of the changes and what features they translate into.
        Keep your description of each feature concise and non-technical, so that a non-technical team member can understand the change in simple terms.
        Avoid listing every commit message or code change. Instead, group the changes into categories like New features, Improvements, Bug fixes, and Dependency changes.
        Avoid describing how a feature will impact a user or experience, just describe what the feature is and what it does.
        Avoid expanding acronyms, for example PLP, PDP or USP, to their full meanings because the users understand those.
        Specify any feature flags connected to these features, if any, along with the environments they are enabled in.
        List any dependency additions, updates or removals that were made in the package.json file only.
    </Instructions>
    <Steps>
        Analyze the Diff: Examine the git code diff to understand the changes in the codebase.
        Analyze Commit Messages: Review the commit messages to gain context and further insights into the changes.
        Identify User-Facing Features: Determine which changes correspond to new features, enhancements, or bug fixes that would be noticeable to the end-users.
        Summarize in Non-Technical Terms: Write a summary of these features in a way that a non-technical team can understand, but no longer than a sentence.
        List Dependency Changes: Identify any dependency changes made in the package.json file (e.g., new libraries, updated versions) and list them.
        Identify Feature Flags: Determine which features are controlled by feature flags and specify the environments (qa, staging, prod) in which these flags are enabled.
        Exclude Unchanged Sections: Only include headings for New features, Improvements, Bug fixes, and Dependency changes if there are updates to list for those headings.
    </Steps>
    <ExampleOutPut1>
        <Output>
            *New features*:
            • Search results can now be filtered by date and relevance.
            • New avatar customisation options have been added to user profiles.
            *Improvements*:
            • Refactored the marketing service to improve readability.
            • Added more breakpoints to the Image component.
            *Bug fixes*:
            • Fixed an issue where the data service was not guarding against unexpected parsing errors.
            • Implemented a workaround to address the caching bug in the user authentication flow.
            *Dependency changes*:
            • Updated Library `XYZ` to version `1.3.0`.
            • Added library `ABC` version `2.1.0`.
        </Output>
    </ExampleOutPut1>
    <ExampleOutPut2>
        <Output>
            *New features*:
            • Added support for tracking URLs in Discord messages for new product discoveries.
        </Output>
    </ExampleOutPut2>
    <ExampleOutPut3>
        <Output>
            *Bug fixes*:
            • Fixed an issue where the Twitter hyperlink was not displaying properly.
        </Output>
    </ExampleOutPut3>
    Please perform this analysis on the provided git code diff and commits and deliver a summary as described above based on that diff.
    The output should be placed in <Output> tags as demonstrated in the examples above.
    Avoid including headings for New features, Improvements, Bug fixes, or Dependency changes if there are no updates to list for those headings.
";
