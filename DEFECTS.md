# Defects Log

Capture defects during manual testing. Each entry will be expanded into a GitHub issue.

---

## Defect: Rustbucket still not dumping logs to S3
- **Functionality**: Logging to S3
- **Issue**: Since our successful manual test of S3 log dumping, no new logs have been dumped in S3.
- **Expected Behavior**: Logs should be dumping every hour.
- **Severity**: High
- **Notes**: Log dumping is one of the most critical-path features of Rustbucket. Without we can't do any registry analysis. 
- **Notes**: We may need to change the way we name the logs to be more granular than YYYYMMDD, or we may overwrite what's in S3.

---

---

## Defect: Log format is overly complex and non-standard
- **Functionality**: Writing logs to filesystem
- **Issue**: Log format should be improved and standardized so that Rustbucket Registry can easily parse it.
- **Expected Behavior**: Well defined, standardized and clear logs.
- **Severity**: High
- **Notes**: Good log format is critical to Rustbucket Registry's analysis functionality.

---