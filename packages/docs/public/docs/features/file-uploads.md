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
  - uploads
  - variables
  - validation
  - response
  - request
  - environment
slug: "/docs/file-uploads"
toc: true
date: "2026-01-24T16:07:59.112Z"
lastModified: "2026-01-24T16:07:59.112Z"
author: "alexvcasillas"
authorUrl: "https://github.com/alexvcasillas/curl-runner"
license: "MIT"
nav:
  label: "File Uploads"
  category: "Documentation"
tags:
  - documentation
  - documentation
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
  datePublished: "2026-01-24T16:07:59.112Z"
  dateModified: "2026-01-24T16:07:59.112Z"
---

# File Uploads

Upload files using multipart/form-data requests with curl-runner.

## Overview

File uploads allow you to send files to APIs using the standard multipart/form-data encoding. This is commonly used for uploading images, documents, and other binary content.

Just specify the file path and curl-runner handles the rest

Override filename and content type as needed

Combine file uploads with regular form fields

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

The `formData` property accepts an object where each key is a form field name. Values can be simple types or file attachments.

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

**file-options.yaml**

```yaml
# File attachment with all options
request:
  name: Upload with Options
  url: https://api.example.com/upload
  method: POST
  formData:
    document:
      file: "./report.pdf"
      filename: "Q4-Report-2024.pdf"
      contentType: "application/pdf"
```

## Complete Examples

### Single File Upload

Upload a single file with validation.

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

Upload multiple files in a single request.

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

Combine file uploads with regular form fields - common for application submissions.

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

### Using Variables

File uploads work seamlessly with curl-runner variables.

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

## Real-World Example

A complete authenticated file upload workflow demonstrating response storage combined with file uploads.

**authenticated-upload.yaml**

```yaml
# Real-world: Image upload with metadata
global:
  execution: sequential
  variables:
    API_URL: https://api.example.com

requests:
  # Step 1: Authenticate
  - name: Login
    url: \${API_URL}/auth/login
    method: POST
    headers:
      Content-Type: application/json
    body:
      username: "admin"
      password: "secret123"
    store:
      authToken: body.accessToken

  # Step 2: Upload image with authentication
  - name: Upload Product Image
    url: \${API_URL}/products/images
    method: POST
    headers:
      Authorization: Bearer \${store.authToken}
    formData:
      product_id: "PROD-12345"
      image_type: "thumbnail"
      image:
        file: "./product-image.jpg"
        filename: "product-thumbnail.jpg"
        contentType: "image/jpeg"
    expect:
      status: 201
    store:
      imageId: body.id

  # Step 3: Verify upload
  - name: Get Image Details
    url: \${API_URL}/images/\${store.imageId}
    method: GET
    headers:
      Authorization: Bearer \${store.authToken}
    expect:
      status: 200
```

## Important Notes

> curl-runner validates that all specified files exist before executing the request. If any file is missing, the request fails with a descriptive error message.

## Best Practices

### Best Practices

• Use descriptive variable names
• Define common values as variables
• Use environment variables for secrets
• Group related variables logically
• Document complex expressions
