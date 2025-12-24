# Defects Log

Capture defects during manual testing. Each entry will be expanded into a GitHub issue.

---

## Defect: Last Log Dump logic missing
- **Where**: /
- **Steps**: Log in to Buckets List ("/"). The "Last Log Dump" column on the homepage always says "none" because there's no logic behind it.
- **Expected**: We should change this to "Last Rustbucket Checkin" because the alternative is polling S3 and trying to pull the timestamp of the last time the rustbucket made a log dump.
- **Actual**: "None"
- **Severity**: Medium
- **Notes**: 

---

## Missing Feature: Approve a registered Rustbucket that's in "Review" status
- **Where**: /
- **Issue**: There's no way to change the status of a bucket from Review to Approved. There should also be other statuses.
- **Importance**: High
- **Notes**: We should do a planning session to determine valid statuses and how a rustbucket gets to them.

---

## Missing Feature: View LogSinks is a dummy page
- **Where**: /logsinks
- **Issue**: The system logs, honeypot activity and Log Analysis by Claude are all dummy data. Let's have a design session to create an MVP of this page.
- **Importance**: High

---

## Missing Feature: Honeypot Activity Tab is a dummy tab
- **Where**: /logsinks -> Click "Honeypot Activity" tab
- **Issue**: This is dummy data and probably doesn't match the current format of the Rustbucket logs. We should evaluate real Rustbucket logs and design this tab accordingly.
- **Importance**: High