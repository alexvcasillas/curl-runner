export const basicUploadExample = `# Basic file upload
request:
  name: Upload Document
  url: https://api.example.com/upload
  method: POST
  formData:
    document:
      file: "./report.pdf"`;

export const formDataTypesExample = `# Different form data types
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
      file: "./avatar.png"`;

export const fileOptionsExample = `# File attachment with all options
request:
  name: Upload with Options
  url: https://api.example.com/upload
  method: POST
  formData:
    document:
      file: "./report.pdf"
      filename: "Q4-Report-2024.pdf"
      contentType: "application/pdf"`;

export const singleFileExample = `# Upload a single file
request:
  name: Upload Profile Picture
  url: https://api.example.com/users/123/avatar
  method: POST
  formData:
    avatar:
      file: "./profile-photo.jpg"
      contentType: "image/jpeg"
  expect:
    status: 200`;

export const multiFileExample = `# Upload multiple files
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
    status: 201`;

export const mixedFormExample = `# Combine files with regular form fields
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
    status: 201`;

export const variableUploadExample = `# Use variables in file upload requests
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
    status: 201`;

export const realWorldExample = `# Real-world: Image upload with metadata
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
      status: 200`;
