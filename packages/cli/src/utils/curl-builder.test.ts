import { describe, expect, test } from 'bun:test';
import { CurlBuilder } from './curl-builder';

describe('CurlBuilder', () => {
  describe('buildCommand', () => {
    test('should build basic GET request', () => {
      const command = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'GET',
      });

      expect(command).toContain('curl');
      expect(command).toContain('-X GET');
      expect(command).toContain('"https://example.com/api"');
    });

    test('should build POST request with JSON body', () => {
      const command = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'POST',
        body: { name: 'test' },
      });

      expect(command).toContain('-X POST');
      expect(command).toContain('-d \'{"name":"test"}\'');
      expect(command).toContain('Content-Type: application/json');
    });

    test('should build POST request with form data', () => {
      const command = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          username: 'john',
          age: 30,
        },
      });

      expect(command).toContain('-X POST');
      expect(command).toContain("-F 'username=john'");
      expect(command).toContain("-F 'age=30'");
      expect(command).not.toContain('-d');
    });

    test('should build POST request with file attachment', () => {
      const command = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          document: {
            file: './test.pdf',
          },
        },
      });

      expect(command).toContain("-F 'document=@./test.pdf'");
    });

    test('should build POST request with file attachment and custom filename', () => {
      const command = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          document: {
            file: './test.pdf',
            filename: 'report.pdf',
          },
        },
      });

      expect(command).toContain("-F 'document=@./test.pdf;filename=report.pdf'");
    });

    test('should build POST request with file attachment and content type', () => {
      const command = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          data: {
            file: './data.json',
            contentType: 'application/json',
          },
        },
      });

      expect(command).toContain("-F 'data=@./data.json;type=application/json'");
    });

    test('should build POST request with file attachment including all options', () => {
      const command = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          document: {
            file: './report.pdf',
            filename: 'quarterly-report.pdf',
            contentType: 'application/pdf',
          },
        },
      });

      expect(command).toContain(
        "-F 'document=@./report.pdf;filename=quarterly-report.pdf;type=application/pdf'",
      );
    });

    test('should build POST request with mixed form data and files', () => {
      const command = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          title: 'My Document',
          description: 'Test upload',
          file: {
            file: './document.pdf',
          },
        },
      });

      expect(command).toContain("-F 'title=My Document'");
      expect(command).toContain("-F 'description=Test upload'");
      expect(command).toContain("-F 'file=@./document.pdf'");
    });

    test('should escape single quotes in form field values', () => {
      const command = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          message: "It's a test",
        },
      });

      expect(command).toContain("-F 'message=It'\\''s a test'");
    });

    test('should prefer formData over body when both are present', () => {
      const command = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'POST',
        formData: {
          field: 'value',
        },
        body: { name: 'test' },
      });

      expect(command).toContain("-F 'field=value'");
      expect(command).not.toContain('-d');
    });

    test('should handle boolean form field values', () => {
      const command = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'POST',
        formData: {
          active: true,
          disabled: false,
        },
      });

      expect(command).toContain("-F 'active=true'");
      expect(command).toContain("-F 'disabled=false'");
    });
  });
});
