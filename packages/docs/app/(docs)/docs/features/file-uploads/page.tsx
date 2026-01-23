import { AlertTriangle, CheckCircle, File, FileText, FolderOpen, Upload } from 'lucide-react';
import type { Metadata } from 'next';
import { CodeBlockServer } from '@/components/code-block-server';
import { H2, H3 } from '@/components/docs-heading';
import { DocsPageHeader } from '@/components/docs-page-header';
import { TableOfContents } from '@/components/toc';
import { Badge } from '@/components/ui/badge';
import {
  basicUploadExample,
  fileOptionsExample,
  formDataTypesExample,
  mixedFormExample,
  multiFileExample,
  realWorldExample,
  singleFileExample,
  variableUploadExample,
} from './snippets';

export const metadata: Metadata = {
  title: 'File Uploads',
  description:
    'Upload files using multipart/form-data requests with curl-runner. Send files, images, and documents to APIs.',
  keywords: [
    'curl-runner file upload',
    'multipart form data',
    'file attachment',
    'upload files',
    'form data',
    'API file upload',
    'curl file upload',
    'image upload',
    'document upload',
    'binary upload',
  ],
  openGraph: {
    title: 'File Uploads | curl-runner Documentation',
    description:
      'Upload files using multipart/form-data requests with curl-runner. Send files, images, and documents to APIs.',
    url: 'https://www.curl-runner.com/docs/features/file-uploads',
    type: 'article',
  },
  twitter: {
    card: 'summary_large_image',
    title: 'File Uploads | curl-runner Documentation',
    description: 'Learn how to upload files using multipart/form-data requests with curl-runner.',
  },
  alternates: {
    canonical: 'https://www.curl-runner.com/docs/features/file-uploads',
  },
};

export default function FileUploadsPage() {
  return (
    <main className="relative py-6 lg:gap-10 lg:py-8 xl:grid xl:grid-cols-[1fr_300px]">
      <div className="mx-auto w-full min-w-0">
        <DocsPageHeader
          heading="File Uploads"
          text="Upload files using multipart/form-data requests with curl-runner."
        />

        <div className="space-y-8">
          {/* Overview */}
          <section>
            <H2 id="overview">Overview</H2>
            <p className="text-muted-foreground mb-6">
              File uploads allow you to send files to APIs using the standard multipart/form-data
              encoding. This is commonly used for uploading images, documents, and other binary
              content.
            </p>

            <div className="grid gap-4 sm:grid-cols-3">
              <div className="rounded-lg border p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Badge className="bg-green-500/10 text-green-600 dark:text-green-400 border-green-500/20">
                    Simple
                  </Badge>
                </div>
                <h4 className="font-medium mb-1">Easy Configuration</h4>
                <p className="text-sm text-muted-foreground">
                  Just specify the file path and curl-runner handles the rest
                </p>
              </div>

              <div className="rounded-lg border p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Badge className="bg-blue-500/10 text-blue-600 dark:text-blue-400 border-blue-500/20">
                    Flexible
                  </Badge>
                </div>
                <h4 className="font-medium mb-1">Custom Options</h4>
                <p className="text-sm text-muted-foreground">
                  Override filename and content type as needed
                </p>
              </div>

              <div className="rounded-lg border p-4">
                <div className="flex items-center gap-2 mb-2">
                  <Badge className="bg-purple-500/10 text-purple-600 dark:text-purple-400 border-purple-500/20">
                    Mixed
                  </Badge>
                </div>
                <h4 className="font-medium mb-1">Files + Fields</h4>
                <p className="text-sm text-muted-foreground">
                  Combine file uploads with regular form fields
                </p>
              </div>
            </div>
          </section>

          {/* Basic Usage */}
          <section>
            <H2 id="basic-usage">Basic Usage</H2>
            <p className="text-muted-foreground mb-6">
              Use the <code className="text-sm bg-muted px-1 py-0.5 rounded">formData</code>{' '}
              property instead of <code className="text-sm bg-muted px-1 py-0.5 rounded">body</code>{' '}
              to send multipart/form-data requests.
            </p>

            <CodeBlockServer language="yaml" filename="basic-upload.yaml">
              {basicUploadExample}
            </CodeBlockServer>
          </section>

          {/* Form Data Types */}
          <section>
            <H2 id="form-data-types">Form Data Configuration</H2>
            <p className="text-muted-foreground mb-6">
              The <code className="text-sm bg-muted px-1 py-0.5 rounded">formData</code> property
              accepts an object where each key is a form field name. Values can be simple types or
              file attachments.
            </p>

            <div className="border rounded-lg overflow-hidden mb-6">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Value Type</th>
                    <th className="text-left p-3 font-medium">Description</th>
                    <th className="text-left p-3 font-medium">Example</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">string</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">Text form field</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      <code>username: "john"</code>
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">number</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">Numeric form field</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      <code>age: 30</code>
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">boolean</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">Boolean form field</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      <code>subscribe: true</code>
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">object</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">File attachment</td>
                    <td className="p-3 text-sm text-muted-foreground">
                      <code>{'{ file: "./doc.pdf" }'}</code>
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>

            <CodeBlockServer language="yaml" filename="form-data-types.yaml">
              {formDataTypesExample}
            </CodeBlockServer>
          </section>

          {/* File Attachment Options */}
          <section>
            <H2 id="file-options">File Attachment Options</H2>
            <p className="text-muted-foreground mb-6">
              File attachments support additional options for customizing how files are sent.
            </p>

            <div className="border rounded-lg overflow-hidden mb-6">
              <table className="w-full">
                <thead className="bg-muted/50">
                  <tr className="border-b">
                    <th className="text-left p-3 font-medium">Property</th>
                    <th className="text-left p-3 font-medium">Type</th>
                    <th className="text-left p-3 font-medium">Required</th>
                    <th className="text-left p-3 font-medium">Description</th>
                  </tr>
                </thead>
                <tbody>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">file</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm">
                      <Badge variant="destructive" className="text-xs">
                        Required
                      </Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Path to the file (relative or absolute)
                    </td>
                  </tr>
                  <tr className="border-b">
                    <td className="p-3">
                      <code className="text-sm">filename</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm">
                      <Badge variant="secondary" className="text-xs">
                        Optional
                      </Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      Custom filename sent to the server
                    </td>
                  </tr>
                  <tr>
                    <td className="p-3">
                      <code className="text-sm">contentType</code>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">string</td>
                    <td className="p-3 text-sm">
                      <Badge variant="secondary" className="text-xs">
                        Optional
                      </Badge>
                    </td>
                    <td className="p-3 text-sm text-muted-foreground">
                      MIME type (curl auto-detects if not specified)
                    </td>
                  </tr>
                </tbody>
              </table>
            </div>

            <CodeBlockServer language="yaml" filename="file-options.yaml">
              {fileOptionsExample}
            </CodeBlockServer>
          </section>

          {/* Examples */}
          <section>
            <H2 id="examples">Complete Examples</H2>

            <div className="space-y-6">
              <div>
                <H3 id="single-file">Single File Upload</H3>
                <p className="text-muted-foreground mb-4">Upload a single file with validation.</p>
                <CodeBlockServer language="yaml" filename="single-file.yaml">
                  {singleFileExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3 id="multiple-files">Multiple File Upload</H3>
                <p className="text-muted-foreground mb-4">
                  Upload multiple files in a single request.
                </p>
                <CodeBlockServer language="yaml" filename="multi-file.yaml">
                  {multiFileExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3 id="mixed-form">Files with Form Fields</H3>
                <p className="text-muted-foreground mb-4">
                  Combine file uploads with regular form fields - common for application
                  submissions.
                </p>
                <CodeBlockServer language="yaml" filename="mixed-form.yaml">
                  {mixedFormExample}
                </CodeBlockServer>
              </div>

              <div>
                <H3 id="with-variables">Using Variables</H3>
                <p className="text-muted-foreground mb-4">
                  File uploads work seamlessly with curl-runner variables.
                </p>
                <CodeBlockServer language="yaml" filename="variable-upload.yaml">
                  {variableUploadExample}
                </CodeBlockServer>
              </div>
            </div>
          </section>

          {/* Real World Example */}
          <section>
            <H2 id="real-world">Real-World Example</H2>
            <p className="text-muted-foreground mb-6">
              A complete authenticated file upload workflow demonstrating response storage combined
              with file uploads.
            </p>

            <CodeBlockServer language="yaml" filename="authenticated-upload.yaml">
              {realWorldExample}
            </CodeBlockServer>
          </section>

          {/* Important Notes */}
          <section>
            <H2 id="important-notes">Important Notes</H2>

            <div className="space-y-4">
              <div className="rounded-lg border bg-yellow-500/5 dark:bg-yellow-500/10 border-yellow-500/20 p-4">
                <div className="flex items-start gap-3">
                  <AlertTriangle className="h-5 w-5 text-yellow-600 dark:text-yellow-400 mt-0.5" />
                  <div>
                    <h4 className="font-medium text-yellow-600 dark:text-yellow-400 mb-1">
                      File Validation
                    </h4>
                    <p className="text-sm text-muted-foreground">
                      curl-runner validates that all specified files exist before executing the
                      request. If any file is missing, the request fails with a descriptive error
                      message.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-muted/50 p-4">
                <div className="flex items-start gap-3">
                  <FolderOpen className="h-5 w-5 text-muted-foreground mt-0.5" />
                  <div>
                    <h4 className="font-medium mb-1">File Path Resolution</h4>
                    <p className="text-sm text-muted-foreground">
                      File paths are resolved relative to the current working directory when running
                      curl-runner. Use absolute paths for files in fixed locations.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-muted/50 p-4">
                <div className="flex items-start gap-3">
                  <FileText className="h-5 w-5 text-muted-foreground mt-0.5" />
                  <div>
                    <h4 className="font-medium mb-1">FormData vs Body</h4>
                    <p className="text-sm text-muted-foreground">
                      <code className="text-sm bg-muted px-1 py-0.5 rounded">formData</code> and{' '}
                      <code className="text-sm bg-muted px-1 py-0.5 rounded">body</code> are
                      mutually exclusive. When both are specified,{' '}
                      <code className="text-sm bg-muted px-1 py-0.5 rounded">formData</code> takes
                      precedence.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>

          {/* Best Practices */}
          <section>
            <H2 id="best-practices">Best Practices</H2>
            <div className="grid gap-4 md:grid-cols-2">
              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-green-500/10 p-2">
                    <CheckCircle className="h-4 w-4 text-green-600 dark:text-green-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use Descriptive Field Names</h4>
                    <p className="text-sm text-muted-foreground">
                      Match field names to what the API expects:{' '}
                      <code className="text-xs">avatar</code>,{' '}
                      <code className="text-xs">document</code>,{' '}
                      <code className="text-xs">attachment</code>.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-blue-500/10 p-2">
                    <File className="h-4 w-4 text-blue-600 dark:text-blue-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Specify Content Types</h4>
                    <p className="text-sm text-muted-foreground">
                      For non-standard file formats, explicitly set the{' '}
                      <code className="text-xs">contentType</code> to ensure proper handling.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-purple-500/10 p-2">
                    <Upload className="h-4 w-4 text-purple-600 dark:text-purple-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Use Relative Paths</h4>
                    <p className="text-sm text-muted-foreground">
                      For portability, use relative paths and run curl-runner from a consistent
                      working directory.
                    </p>
                  </div>
                </div>
              </div>

              <div className="rounded-lg border bg-card p-4">
                <div className="flex items-start gap-3">
                  <div className="rounded-full bg-orange-500/10 p-2">
                    <AlertTriangle className="h-4 w-4 text-orange-600 dark:text-orange-400" />
                  </div>
                  <div>
                    <h4 className="font-medium mb-2">Security First</h4>
                    <p className="text-sm text-muted-foreground">
                      Never commit sensitive files to version control. Use environment variables for
                      paths to sensitive data.
                    </p>
                  </div>
                </div>
              </div>
            </div>
          </section>
        </div>
      </div>

      {/* Table of Contents */}
      <div className="hidden text-sm xl:block">
        <div className="sticky top-16 -mt-10 pt-4">
          <TableOfContents />
        </div>
      </div>
    </main>
  );
}
