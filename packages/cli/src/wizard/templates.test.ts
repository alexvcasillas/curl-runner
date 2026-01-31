import { describe, expect, it } from 'bun:test';
import { getTemplate, getTemplateChoices, TEMPLATES } from './templates';

describe('templates', () => {
  describe('TEMPLATES', () => {
    it('has basic-get template', () => {
      expect(TEMPLATES['basic-get']).toBeDefined();
      expect(TEMPLATES['basic-get'].name).toBe('Basic GET Request');
      expect(TEMPLATES['basic-get'].defaults.method).toBe('GET');
    });

    it('has basic-post template', () => {
      expect(TEMPLATES['basic-post']).toBeDefined();
      expect(TEMPLATES['basic-post'].name).toBe('Basic POST Request');
      expect(TEMPLATES['basic-post'].defaults.method).toBe('POST');
      expect(TEMPLATES['basic-post'].defaults.bodyType).toBe('json');
    });

    it('has api-test template', () => {
      expect(TEMPLATES['api-test']).toBeDefined();
      expect(TEMPLATES['api-test'].defaults.expectStatus).toBe(200);
      expect(TEMPLATES['api-test'].defaults.verbose).toBe(true);
    });

    it('has file-upload template', () => {
      expect(TEMPLATES['file-upload']).toBeDefined();
      expect(TEMPLATES['file-upload'].defaults.method).toBe('POST');
      expect(TEMPLATES['file-upload'].defaults.bodyType).toBe('form');
    });

    it('has auth-flow template', () => {
      expect(TEMPLATES['auth-flow']).toBeDefined();
      expect(TEMPLATES['auth-flow'].defaults.authType).toBe('bearer');
    });
  });

  describe('getTemplate', () => {
    it('returns template by name', () => {
      const template = getTemplate('basic-get');

      expect(template.name).toBe('Basic GET Request');
    });

    it('returns correct defaults for basic-post', () => {
      const template = getTemplate('basic-post');

      expect(template.defaults.headers).toEqual({
        'Content-Type': 'application/json',
      });
    });
  });

  describe('getTemplateChoices', () => {
    it('returns array of choices', () => {
      const choices = getTemplateChoices();

      expect(Array.isArray(choices)).toBe(true);
      expect(choices.length).toBe(5);
    });

    it('each choice has value, label, and hint', () => {
      const choices = getTemplateChoices();

      for (const choice of choices) {
        expect(choice.value).toBeDefined();
        expect(choice.label).toBeDefined();
        expect(choice.hint).toBeDefined();
      }
    });

    it('includes basic-get in choices', () => {
      const choices = getTemplateChoices();
      const basicGet = choices.find((c) => c.value === 'basic-get');

      expect(basicGet).toBeDefined();
      expect(basicGet?.label).toBe('Basic GET Request');
    });
  });
});
