---
"@curl-runner/cli": minor
---

Add file attachment support for multipart/form-data requests

New `formData` property for uploading files and sending form data:

- Send files using the `file` property with path to the file
- Customize the filename sent to the server with `filename`
- Specify explicit MIME type with `contentType`
- Mix file attachments with regular form fields
- Automatic file existence validation before request execution

Example usage:

```yaml
request:
  name: Upload Document
  url: https://api.example.com/upload
  method: POST
  formData:
    title: "My Document"
    document:
      file: "./report.pdf"
      filename: "quarterly-report.pdf"
      contentType: "application/pdf"
```
