import { describe, expect, test } from 'bun:test';
import { CurlBuilder } from './curl-builder';

describe('CurlBuilder', () => {
  describe('buildCommand', () => {
    test('should build basic GET request', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'GET',
      });

      expect(args).toContain('-X');
      expect(args).toContain('GET');
      expect(args).toContain('https://example.com/api');
    });

    test('should build POST request with JSON body', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'POST',
        body: { name: 'test' },
      });

      expect(args).toContain('-X');
      expect(args).toContain('POST');
      expect(args).toContain('-d');
      expect(args).toContain('{"name":"test"}');
      expect(args).toContain('-H');
      expect(args).toContain('Content-Type: application/json');
    });

    test('should build POST request with form data', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          username: 'john',
          age: 30,
        },
      });

      expect(args).toContain('-X');
      expect(args).toContain('POST');
      expect(args).toContain('--form-string');
      expect(args).toContain('username=john');
      expect(args).toContain('age=30');
      expect(args).not.toContain('-d');
    });

    test('should build POST request with file attachment', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          document: {
            file: './test.pdf',
          },
        },
      });

      expect(args).toContain('-F');
      expect(args).toContain('document=@./test.pdf');
    });

    test('should build POST request with file attachment and custom filename', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          document: {
            file: './test.pdf',
            filename: 'report.pdf',
          },
        },
      });

      expect(args).toContain('-F');
      expect(args).toContain('document=@./test.pdf;filename=report.pdf');
    });

    test('should build POST request with file attachment and content type', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          data: {
            file: './data.json',
            contentType: 'application/json',
          },
        },
      });

      expect(args).toContain('-F');
      expect(args).toContain('data=@./data.json;type=application/json');
    });

    test('should build POST request with file attachment including all options', () => {
      const args = CurlBuilder.buildCommand({
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

      expect(args).toContain('-F');
      expect(args).toContain(
        'document=@./report.pdf;filename=quarterly-report.pdf;type=application/pdf',
      );
    });

    test('should build POST request with mixed form data and files', () => {
      const args = CurlBuilder.buildCommand({
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

      expect(args).toContain('--form-string');
      expect(args).toContain('title=My Document');
      expect(args).toContain('description=Test upload');
      expect(args).toContain('-F');
      expect(args).toContain('file=@./document.pdf');
    });

    test('should pass through single quotes in form field values', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/upload',
        method: 'POST',
        formData: {
          message: "It's a test",
        },
      });

      expect(args).toContain('--form-string');
      expect(args).toContain("message=It's a test");
    });

    test('should prefer formData over body when both are present', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'POST',
        formData: {
          field: 'value',
        },
        body: { name: 'test' },
      });

      expect(args).toContain('--form-string');
      expect(args).toContain('field=value');
      expect(args).not.toContain('-d');
    });

    test('should handle boolean form field values', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'POST',
        formData: {
          active: true,
          disabled: false,
        },
      });

      expect(args).toContain('--form-string');
      expect(args).toContain('active=true');
      expect(args).toContain('disabled=false');
    });

    test('should pass through unresolved ${VAR} without shell interpretation', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/api/${VAR}',
        method: 'GET',
      });

      // The ${VAR} should be in the URL literally, not interpreted
      expect(args).toContain('https://example.com/api/${VAR}');
    });

    test('should handle JSON body with curly braces', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'POST',
        body: { nested: { key: 'value' } },
      });

      expect(args).toContain('-d');
      expect(args).toContain('{"nested":{"key":"value"}}');
    });

    test('should add --http2 flag when http2 is enabled', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'GET',
        http2: true,
      });

      expect(args).toContain('--http2');
    });

    test('should not add --http2 flag when http2 is false', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'GET',
        http2: false,
      });

      expect(args).not.toContain('--http2');
    });

    test('should not add --http2 flag when http2 is not specified', () => {
      const args = CurlBuilder.buildCommand({
        url: 'https://example.com/api',
        method: 'GET',
      });

      expect(args).not.toContain('--http2');
    });
  });

  describe('formatCommandForDisplay', () => {
    test('should format simple args', () => {
      const display = CurlBuilder.formatCommandForDisplay(['-X', 'GET', 'https://example.com']);
      expect(display).toBe('curl -X GET https://example.com');
    });

    test('should quote args with spaces', () => {
      const display = CurlBuilder.formatCommandForDisplay(['-H', 'Content-Type: application/json']);
      expect(display).toBe("curl -H 'Content-Type: application/json'");
    });

    test('should escape single quotes in args', () => {
      const display = CurlBuilder.formatCommandForDisplay(['--form-string', "message=It's a test"]);
      expect(display).toBe("curl --form-string 'message=It'\\''s a test'");
    });
  });
});
