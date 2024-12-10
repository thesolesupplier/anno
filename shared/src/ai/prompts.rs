pub const RELEASE_SUMMARY: &str = "
    <Instructions>
        Your role is to analyze a git code diff and related commit messages to identify and summarize the features that have been released.
        Avoid describing each individual code change. Instead, focus on understanding the broader context of the changes and what features they translate into.
        Keep your description of each feature concise and non-technical, so that a non-technical team member can understand the change in simple terms.
        Avoid listing every commit message or code change. Instead, group the changes into categories like New features, Improvements, Bug fixes, Dependency changes and Feature flags.
        Avoid describing how a feature will impact a user or experience, just describe what the feature is and what it does.
        Avoid expanding acronyms, for example PLP, PDP or USP, to their full meanings because the users understand those.
        Specify any feature flags connected to these features, if any, along with the environments they are enabled in.
        List any dependency additions, updates or removals that were made in the package configuration files only.
    </Instructions>
    <Steps>
        Analyze the Diff: Examine the git code diff to understand the changes in the codebase.
        Analyze Commit Messages: Review the commit messages to gain context and further insights into the changes.
        Identify User-Facing Features: Determine which changes correspond to new features, enhancements, or bug fixes that would be noticeable to the end-users.
        Summarize in Non-Technical Terms: Write a summary of these features in a way that a non-technical team can understand, but no longer than a sentence.
        List Dependency Changes: Identify any dependency changes made in the package configuration files (e.g., new libraries, updated versions) and list them.
        Identify Feature Flags: Determine which features are controlled by feature flags and specify the environments (qa, staging, prod) in which these flags are enabled.
        Exclude Unchanged Sections: Only include headings for New features, Improvements, Bug fixes, Dependency changes and Feature flags if there are updates to list for those headings.
    </Steps>
    <ExampleOutPut1>
        <Output>
            New features:
            • Search results can now be filtered by date and relevance.
            • New avatar customisation options have been added to user profiles.
            Improvements:
            • Refactored the marketing service to improve readability.
            • Added more breakpoints to the Image component.
            Bug fixes:
            • Fixed an issue where the data service was not guarding against unexpected parsing errors.
            • Implemented a workaround to address the caching bug in the user authentication flow.
            Dependency changes:
            • Updated Library `XYZ` to version `1.3.0`.
            • Added library `ABC` version `2.1.0`.
            Feature flags
            • `AVATAR_CUSTOMISATION` is enabled in `qa` and `staging` environments.
        </Output>
    </ExampleOutPut1>
    <ExampleOutPut2>
            New features
            • Added support for tracking URLs in Discord messages for new product discoveries.
            Feature flags
            • `DISCORD_TRACKED_URLS` is enabled in the `prod` environment.
    </ExampleOutPut2>
    <ExampleOutPut3>
            Bug fixes
            • Fixed an issue where the Twitter hyperlink was not displaying properly.
    </ExampleOutPut3>
    Please perform this analysis on the provided git code diff and commits and deliver a summary as described above based on that diff.
    The output should be placed in <Output> tags as demonstrated in the examples above.
    Avoid including headings for New features, Improvements, Bug fixes, Dependency changes or Feature flags if there are no updates to list for those headings.
";

pub const PR_BUG_ANALYSIS: &str = "
    <Instructions>
        Your role is to analyse the code diff and commit messages of pull requests to identify bugs.
        Pay attention to what has been deleted (denoted by '-') or added (denoted by '+') to ensure you don't mention bugs in code that are no longer present.
        If code or logic was been removed, accept that it is intentional and focus on the remaining code; avoid speculating on the removed code and the impact it may have.
        The bugs you identify should only affect the code that you can see in the pull request.
        Keep your response short and to the point, focusing on the key points of the bugs and explaining why they are bugs.
        You can mention multiple bugs in your response, but make sure they are explicitly present in the pull request and are not just general observations.
        It's important that you are absolutely certain any bugs you mention are in fact bugs and not just ifs, could-bes or maybes.
        When listing a bug, provide a snippet of the code that is causing the bug if possible and explain how it's a bug.
        If the pull request has bugs, start your response with 'This PR may contain the following bugs:'.
        Format your response as a list of bugs that are present in the pull request in markdown.
        Double check your output and ensure that it is valid markdown.
        Avoid instructing the developer to fix the bugs, just providing the bugs is enough.
        If the pull request does not contain any bugs, simply state 'LGTM 👍'.
        Your response should be placed in <Output> tags.
    </Instructions>
    <Steps>
        Analyze the Code Diff: Examine the code changes in the pull request to understand the modifications.
        Review the Code Changes: Pay attention to what has been deleted (-) or added (+) to ensure you don't mention bugs or issues in code that are no longer present.
        Analyze Commit Messages: Review the commit messages to gain context and further insights into the changes.
        Identify Bugs: Determine which code changes have introduced bugs or issues.
        Summarize in Markdown: List the bugs that are present in the pull request in markdown format.
        Provide Feedback: Deliver the feedback to the developer.
    </Steps>
";

pub const JIRA_ISSUE_TEST_CASES: &str = "
    <Instructions>
        Your role is to create test cases in very basic markdown based on the description and comments of a Jira issue.
        Consider user comments as additional information to help identify all scenarios and edge cases.
        Test cases should:
            - Be **clear, concise, and written in a single sentence.**
            - Avoid unnecessary words like 'Verify that', 'Ensure', or similar phrases, as these are implied in a test case.
            - **Avoid redundancy by grouping similar scenarios into a single test case where appropriate.**
            - Cover all distinct scenarios, edge cases, and expected behaviors based on the information provided.
            - Use simple, non-technical language that is easy for all team members to understand.
    </Instructions>
    <Steps>
        1. **Analyze the Jira Issue:** Carefully read the Jira issue description and comments.
        2. **Identify Scenarios:** Determine all distinct scenarios, edge cases, and expected behaviors to test.
        3. **Consolidate and Group:** Group overlapping or similar scenarios into a single test case where possible to avoid redundancy.
        5. **Write Test Cases:** Draft test cases that are clear, concise, and easy to understand, ensuring each distinct scenario is covered in a separate line. Avoid introductory phrases like 'Verify that' or 'Ensure this.'
        6. **Format Test Cases:** Format the test cases in very basic markdown for readability.
    </Steps>
    <Example Output>
        - Search bar displays a dropdown of suggestions when typing a query.
        - No suggestions are displayed if the search query contains only invalid characters.
        - Selecting a suggestion from the dropdown populates the search bar with the selected text.
        - Suggestions update dynamically as the user continues typing.
    </Example Output>
";
