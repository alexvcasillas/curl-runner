---
title: "File Uploads"
description: "Upload files using multipart/form-data requests with curl-runner."
category: "Documentation"
keywords:
  - curl-runner
  - http
  - api
  - testing
  - file
  - upload
  - multipart
  - form-data
  - attachment
slug: "/docs/file-uploads"
toc: true
date: "2026-01-23T21:27:49.063Z"
lastModified: "2026-01-23T21:27:49.063Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "File Uploads"
  category: "Documentation"
tags:
  - documentation
  - features
og:
  title: "File Uploads - curl-runner Documentation"
  description: "Upload files using multipart/form-data requests with curl-runner."
  type: "article"
  image: "/og-image.png"
schema:
  "@context": "https://schema.org"
  "@type": "TechArticle"
  headline: "File Uploads"
  description: "Upload files using multipart/form-data requests with curl-runner."
  datePublished: "2026-01-23T21:27:49.063Z"
  dateModified: "2026-01-23T21:27:49.063Z"
---

# File Uploads

Upload files using multipart/form-data requests with curl-runner.

## Overview

File uploads allow you to send files to APIs using the standard multipart/form-data encoding. This is commonly used for:

- Uploading images, documents, and other files
- Submitting forms with file attachments
- Sending mixed data (files and text fields) in a single request

## Basic Usage

Use the `formData` property instead of `body` to send multipart/form-data requests.

**basic-upload.yaml**

```yaml
# Basic file upload
request:
  name: Upload Document
  url: https://api.example.com/upload
  method: POST
  formData:
    document:
      file: "./report.pdf"
```

## Form Data Configuration

The `formData` property accepts an object where each key is a form field name. Values can be:

- **Simple values**: strings, numbers, or booleans
- **File attachments**: objects with a `file` property

**form-data-types.yaml**

```yaml
# Different form data types
request:
  name: Mixed Form Data
  url: https://api.example.com/submit
  method: POST
  formData:
    # Simple string value
    username: "john_doe"

    # Number value
    age: 30

    # Boolean value
    subscribe: true

    # File attachment
    avatar:
      file: "./avatar.png"
```

## File Attachment Options

File attachments support additional options for customizing how files are sent.

### Basic File Attachment

The simplest form just specifies the file path:

```yaml
formData:
  document:
    file: "./path/to/file.pdf"
```

### Custom Filename

Override the filename sent to the server:

```yaml
formData:
  document:
    file: "./local-file.pdf"
    filename: "quarterly-report.pdf"
```

### Explicit Content Type

Specify the MIME type when curl cannot auto-detect it:

```yaml
formData:
  data:
    file: "./data.bin"
    contentType: "application/octet-stream"
```

### All Options Combined

```yaml
formData:
  document:
    file: "./report.pdf"
    filename: "Q4-Report-2024.pdf"
    contentType: "application/pdf"
```

## Complete Examples

### Single File Upload

**single-file.yaml**

```yaml
# Upload a single file
request:
  name: Upload Profile Picture
  url: https://api.example.com/users/123/avatar
  method: POST
  formData:
    avatar:
      file: "./profile-photo.jpg"
      contentType: "image/jpeg"
  expect:
    status: 200
```

### Multiple File Upload

**multi-file.yaml**

```yaml
# Upload multiple files
request:
  name: Upload Documents
  url: https://api.example.com/documents/batch
  method: POST
  formData:
    primary_document:
      file: "./main-report.pdf"
      filename: "report.pdf"
      contentType: "application/pdf"

    supporting_document:
      file: "./appendix.pdf"
      filename: "appendix.pdf"
      contentType: "application/pdf"

    spreadsheet:
      file: "./data.xlsx"
      contentType: "application/vnd.openxmlformats-officedocument.spreadsheetml.sheet"
  expect:
    status: 201
```

### Files with Form Fields

**mixed-form.yaml**

```yaml
# Combine files with regular form fields
request:
  name: Submit Application
  url: https://api.example.com/applications
  method: POST
  formData:
    # Text fields
    applicant_name: "John Doe"
    email: "john@example.com"
    position: "Software Engineer"

    # File attachments
    resume:
      file: "./resume.pdf"
      filename: "john-doe-resume.pdf"
      contentType: "application/pdf"

    cover_letter:
      file: "./cover-letter.pdf"
      contentType: "application/pdf"
  expect:
    status: 201
```

### Using Variables in File Uploads

**variable-upload.yaml**

```yaml
# Use variables in file upload requests
global:
  variables:
    API_URL: https://api.example.com
    USER_ID: "12345"

request:
  name: Upload User Document
  url: \${API_URL}/users/\${USER_ID}/documents
  method: POST
  formData:
    description: "Uploaded on \${DATE:YYYY-MM-DD}"
    request_id: "\${UUID}"
    document:
      file: "./document.pdf"
  expect:
    status: 201
```

## File Path Resolution

File paths in `formData` are resolved relative to the current working directory when running curl-runner.

```bash
# If running from project root:
curl-runner examples/file-upload.yaml

# Files are resolved relative to where you run the command
# ./avatar.png would look for avatar.png in the current directory
```

### Tips for File Paths

- Use relative paths for portability
- Use absolute paths when files are in fixed locations
- Consider the working directory when running curl-runner

## Error Handling

curl-runner validates that all specified files exist before executing the request. If any file is missing, the request fails with a descriptive error.

```
Error: File(s) not found: avatar: ./missing-file.png, document: ./another-missing.pdf
```

## FormData vs Body

The `formData` and `body` properties are mutually exclusive. When both are specified, `formData` takes precedence.

| Use Case | Property |
|----------|----------|
| JSON data | `body` |
| Plain text | `body` |
| File uploads | `formData` |
| Form submissions with files | `formData` |
| Mixed files and fields | `formData` |

## Best Practices

### File Upload Best Practices

- Use descriptive field names that match API expectations
- Specify content types for non-standard file formats
- Use custom filenames when the original name isn't meaningful
- Validate file existence before running large test suites
- Use variables for dynamic file paths when needed

### Security Considerations

- Never commit sensitive files to version control
- Use environment variables for file paths containing sensitive data
- Be cautious with file uploads in CI/CD pipelines
