/**
 * YAML Wizard - Interactive CLI for creating curl-runner YAML files.
 */

import * as p from '@clack/prompts';
import { getVersion } from '../version';
import { fileExists, loadYamlFile } from './parser';
import {
  promptAdvanced,
  promptAuth,
  promptBasics,
  promptBody,
  promptHeaders,
  promptOutput,
  promptRetry,
  promptTemplate,
  promptValidation,
} from './prompts';
import { getTemplate } from './templates';
import type { WizardAnswers, WizardOptions, WizardTemplate } from './types';
import { buildYamlConfig, toYamlString } from './yaml-builder';

export type { WizardAnswers, WizardOptions } from './types';

/**
 * Main wizard entry point.
 */
export async function runWizard(options: WizardOptions = {}): Promise<void> {
  p.intro(`curl-runner wizard v${getVersion()}`);

  let answers: WizardAnswers;

  // Edit mode - load existing file
  if (options.editFile) {
    if (!(await fileExists(options.editFile))) {
      p.cancel(`File not found: ${options.editFile}`);
      process.exit(1);
    }

    p.log.info(`Editing: ${options.editFile}`);
    const existing = await loadYamlFile(options.editFile);
    answers = await runPromptFlow(existing);
    options.outputPath = options.outputPath || options.editFile;
  } else {
    // New file - start from template or blank
    const template = await promptTemplate();

    if (p.isCancel(template)) {
      p.cancel('Wizard cancelled');
      process.exit(0);
    }

    const defaults =
      template === 'blank' ? undefined : getTemplate(template as WizardTemplate).defaults;

    answers = await runPromptFlow(defaults as Partial<WizardAnswers> | undefined);
  }

  // Output options
  const output = await promptOutput({
    outputFile: options.outputPath,
  });

  if (p.isCancel(output)) {
    p.cancel('Wizard cancelled');
    process.exit(0);
  }

  // Build YAML
  const config = buildYamlConfig(answers);
  const yaml = toYamlString(config);

  // Preview
  p.log.step('Preview:');
  console.log();
  console.log(yaml);
  console.log();

  // Confirm save
  const confirm = await p.confirm({
    message: `Save to ${output.outputPath}?`,
    initialValue: true,
  });

  if (p.isCancel(confirm) || !confirm) {
    p.cancel('Wizard cancelled');
    process.exit(0);
  }

  // Write file
  await Bun.write(output.outputPath, yaml);
  p.log.success(`Saved to ${output.outputPath}`);

  // Run after create
  if (output.runAfter) {
    p.log.step('Running request...');
    console.log();

    const proc = Bun.spawn(['bun', 'run', process.argv[1], output.outputPath], {
      stdout: 'inherit',
      stderr: 'inherit',
    });

    await proc.exited;
  }

  p.outro('Done!');
}

/**
 * Runs the full prompt flow.
 */
async function runPromptFlow(defaults?: Partial<WizardAnswers>): Promise<WizardAnswers> {
  // Step 1: Basics (URL, method, name)
  const basics = await promptBasics(defaults);
  if (p.isCancel(basics)) {
    p.cancel('Wizard cancelled');
    process.exit(0);
  }

  let answers: WizardAnswers = {
    ...defaults,
    ...basics,
  } as WizardAnswers;

  // Step 2: Headers
  const headers = await promptHeaders(defaults?.headers);
  if (p.isCancel(headers)) {
    p.cancel('Wizard cancelled');
    process.exit(0);
  }
  if (headers) {
    answers.headers = headers;
  }

  // Step 3: Body (skip for GET/HEAD/OPTIONS)
  if (!['GET', 'HEAD', 'OPTIONS'].includes(answers.method)) {
    const body = await promptBody(defaults);
    if (p.isCancel(body)) {
      p.cancel('Wizard cancelled');
      process.exit(0);
    }
    answers = { ...answers, ...body };
  }

  // Step 4: Auth
  const auth = await promptAuth(defaults);
  if (p.isCancel(auth)) {
    p.cancel('Wizard cancelled');
    process.exit(0);
  }
  answers = { ...answers, ...auth };

  // Step 5: Advanced options
  const advanced = await promptAdvanced(defaults);
  if (p.isCancel(advanced)) {
    p.cancel('Wizard cancelled');
    process.exit(0);
  }
  answers = { ...answers, ...advanced };

  // Step 6: Retry
  const retry = await promptRetry(defaults);
  if (p.isCancel(retry)) {
    p.cancel('Wizard cancelled');
    process.exit(0);
  }
  answers = { ...answers, ...retry };

  // Step 7: Validation
  const validation = await promptValidation(defaults);
  if (p.isCancel(validation)) {
    p.cancel('Wizard cancelled');
    process.exit(0);
  }
  answers = { ...answers, ...validation };

  return answers;
}

/**
 * Quick init - creates a simple request without full wizard.
 */
export async function quickInit(url?: string): Promise<void> {
  p.intro('curl-runner quick init');

  const urlInput =
    url ||
    (await p.text({
      message: 'Request URL',
      placeholder: 'https://api.example.com/endpoint',
      validate: (value) => {
        if (!value) {
          return 'URL is required';
        }
        try {
          new URL(value);
        } catch {
          return 'Invalid URL format';
        }
      },
    }));

  if (p.isCancel(urlInput)) {
    p.cancel('Cancelled');
    process.exit(0);
  }

  const method = await p.select({
    message: 'HTTP method',
    initialValue: 'GET' as const,
    options: [
      { value: 'GET' as const, label: 'GET' },
      { value: 'POST' as const, label: 'POST' },
      { value: 'PUT' as const, label: 'PUT' },
      { value: 'DELETE' as const, label: 'DELETE' },
    ],
  });

  if (p.isCancel(method)) {
    p.cancel('Cancelled');
    process.exit(0);
  }

  const filename = await p.text({
    message: 'Save to',
    placeholder: 'request.yaml',
    initialValue: 'request.yaml',
    validate: (v) => {
      if (!v) {
        return 'Filename required';
      }
      if (!v.endsWith('.yaml') && !v.endsWith('.yml')) {
        return 'Must end with .yaml or .yml';
      }
    },
  });

  if (p.isCancel(filename)) {
    p.cancel('Cancelled');
    process.exit(0);
  }

  const config = buildYamlConfig({
    url: urlInput,
    method: method,
  });

  const yaml = toYamlString(config);

  await Bun.write(filename, yaml);
  p.log.success(`Created ${filename}`);

  const run = await p.confirm({
    message: 'Run now?',
    initialValue: true,
  });

  if (p.isCancel(run)) {
    p.outro('Done!');
    return;
  }

  if (run) {
    console.log();
    const proc = Bun.spawn(['bun', 'run', process.argv[1], filename], {
      stdout: 'inherit',
      stderr: 'inherit',
    });
    await proc.exited;
  }

  p.outro('Done!');
}
