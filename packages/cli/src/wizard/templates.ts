/**
 * Wizard templates for common use cases.
 */

import type { TemplateConfig, WizardTemplate } from './types';

export const TEMPLATES: Record<WizardTemplate, TemplateConfig> = {
  'basic-get': {
    name: 'Basic GET Request',
    description: 'Simple GET request to fetch data',
    defaults: {
      method: 'GET',
      followRedirects: true,
    },
  },
  'basic-post': {
    name: 'Basic POST Request',
    description: 'POST request with JSON body',
    defaults: {
      method: 'POST',
      bodyType: 'json',
      headers: { 'Content-Type': 'application/json' },
    },
  },
  'api-test': {
    name: 'API Test',
    description: 'Request with validation and response time check',
    defaults: {
      method: 'GET',
      expectStatus: 200,
      expectResponseTime: '< 2000',
      verbose: true,
      showHeaders: true,
    },
  },
  'file-upload': {
    name: 'File Upload',
    description: 'Multipart form upload request',
    defaults: {
      method: 'POST',
      bodyType: 'form',
    },
  },
  'auth-flow': {
    name: 'Auth Flow',
    description: 'Request with authentication',
    defaults: {
      method: 'POST',
      authType: 'bearer',
      headers: { 'Content-Type': 'application/json' },
    },
  },
};

export function getTemplate(name: WizardTemplate): TemplateConfig {
  return TEMPLATES[name];
}

export function getTemplateChoices(): { value: WizardTemplate; label: string; hint: string }[] {
  return Object.entries(TEMPLATES).map(([key, config]) => ({
    value: key as WizardTemplate,
    label: config.name,
    hint: config.description,
  }));
}
