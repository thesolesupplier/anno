# **Anno**

Anno is a Github deployment **anno**tator in the form of a Rust API that receives status updates of deployments via a webhook, compares the git diff between the last successful deployment and the current and posts to slack a quick AI generated summary of the change that's just been deployed.

