#!/usr/bin/env bun

import { Glob } from 'bun';
import { RequestExecutor } from './executor/request-executor';
import { YamlParser } from './parser/yaml';
import type { GlobalConfig, RequestConfig } from './types/config';
import { Logger } from './utils/logger';

class CurlRunnerCLI {
  private logger = new Logger();

  async run(args: string[]): Promise<void> {
    try {
      const { files, options } = this.parseArguments(args);

      if (options.help) {
        this.showHelp();
        return;
      }

      if (options.version) {
        console.log('curl-runner v1.0.0');
        return;
      }

      const yamlFiles = await this.findYamlFiles(files, options);

      if (yamlFiles.length === 0) {
        this.logger.logError('No YAML files found');
        process.exit(1);
      }

      this.logger.logInfo(`Found ${yamlFiles.length} YAML file(s)`);

      const allRequests: RequestConfig[] = [];
      let globalConfig: GlobalConfig = {};

      for (const file of yamlFiles) {
        this.logger.logInfo(`Processing: ${file}`);
        const { requests, config } = await this.processYamlFile(file);

        if (config) {
          globalConfig = this.mergeGlobalConfigs(globalConfig, config);
        }

        allRequests.push(...requests);
      }

      if (options.execution) {
        globalConfig.execution = options.execution as 'sequential' | 'parallel';
      }
      if (options.continueOnError !== undefined) {
        globalConfig.continueOnError = options.continueOnError;
      }
      if (options.verbose !== undefined) {
        globalConfig.output = { ...globalConfig.output, verbose: options.verbose };
      }
      if (options.output) {
        globalConfig.output = { ...globalConfig.output, saveToFile: options.output };
      }

      if (allRequests.length === 0) {
        this.logger.logError('No requests found in YAML files');
        process.exit(1);
      }

      const executor = new RequestExecutor(globalConfig);
      const summary = await executor.execute(allRequests);

      process.exit(summary.failed > 0 && !globalConfig.continueOnError ? 1 : 0);
    } catch (error) {
      this.logger.logError(error instanceof Error ? error.message : String(error));
      process.exit(1);
    }
  }

  private parseArguments(args: string[]): { files: string[]; options: Record<string, unknown> } {
    const options: Record<string, unknown> = {};
    const files: string[] = [];

    for (let i = 0; i < args.length; i++) {
      const arg = args[i];

      if (arg.startsWith('--')) {
        const key = arg.slice(2);
        const nextArg = args[i + 1];

        if (key === 'help' || key === 'version') {
          options[key] = true;
        } else if (nextArg && !nextArg.startsWith('--')) {
          if (key === 'continue-on-error') {
            options.continueOnError = nextArg === 'true';
          } else if (key === 'verbose') {
            options.verbose = nextArg === 'true';
          } else {
            options[key] = nextArg;
          }
          i++;
        } else {
          options[key] = true;
        }
      } else if (arg.startsWith('-')) {
        const flags = arg.slice(1);
        for (const flag of flags) {
          switch (flag) {
            case 'h':
              options.help = true;
              break;
            case 'v':
              options.verbose = true;
              break;
            case 'p':
              options.execution = 'parallel';
              break;
            case 'c':
              options.continueOnError = true;
              break;
          }
        }
      } else {
        files.push(arg);
      }
    }

    return { files, options };
  }

  private async findYamlFiles(
    patterns: string[],
    options: Record<string, unknown>,
  ): Promise<string[]> {
    const files: Set<string> = new Set();

    let searchPatterns: string[] = [];

    if (patterns.length === 0) {
      searchPatterns = options.all
        ? ['**/*.yaml', '**/*.yml']
        : ['*.yaml', '*.yml'];
    } else {
      // Check if patterns include directories
      for (const pattern of patterns) {
        try {
          // Use Bun's file system API to check if it's a directory
          const fs = await import('fs/promises');
          const stat = await fs.stat(pattern);
          
          if (stat.isDirectory()) {
            // Add glob patterns for all YAML files in this directory
            searchPatterns.push(`${pattern}/*.yaml`, `${pattern}/*.yml`);
            // If --all flag is set, search recursively
            if (options.all) {
              searchPatterns.push(`${pattern}/**/*.yaml`, `${pattern}/**/*.yml`);
            }
          } else if (stat.isFile()) {
            // It's a file, add it directly
            searchPatterns.push(pattern);
          }
        } catch {
          // If stat fails, assume it's a glob pattern
          searchPatterns.push(pattern);
        }
      }
    }

    for (const pattern of searchPatterns) {
      const globber = new Glob(pattern);
      for await (const file of globber.scan('.')) {
        // Only add files with .yaml or .yml extension
        if (file.endsWith('.yaml') || file.endsWith('.yml')) {
          files.add(file);
        }
      }
    }

    return Array.from(files).sort();
  }

  private async processYamlFile(
    filepath: string,
  ): Promise<{ requests: RequestConfig[]; config?: GlobalConfig }> {
    const yamlContent = await YamlParser.parseFile(filepath);
    const requests: RequestConfig[] = [];
    let globalConfig: GlobalConfig | undefined;

    if (yamlContent.global) {
      globalConfig = yamlContent.global;
    }

    const variables = {
      ...yamlContent.global?.variables,
      ...yamlContent.collection?.variables,
    };

    const defaults = {
      ...yamlContent.global?.defaults,
      ...yamlContent.collection?.defaults,
    };

    if (yamlContent.request) {
      const request = this.prepareRequest(yamlContent.request, variables, defaults);
      requests.push(request);
    }

    if (yamlContent.requests) {
      for (const req of yamlContent.requests) {
        const request = this.prepareRequest(req, variables, defaults);
        requests.push(request);
      }
    }

    if (yamlContent.collection?.requests) {
      for (const req of yamlContent.collection.requests) {
        const request = this.prepareRequest(req, variables, defaults);
        requests.push(request);
      }
    }

    return { requests, config: globalConfig };
  }

  private prepareRequest(
    request: RequestConfig,
    variables: Record<string, string>,
    defaults: Partial<RequestConfig>,
  ): RequestConfig {
    const interpolated = YamlParser.interpolateVariables(request, variables);
    return YamlParser.mergeConfigs(defaults, interpolated);
  }

  private mergeGlobalConfigs(base: GlobalConfig, override: GlobalConfig): GlobalConfig {
    return {
      ...base,
      ...override,
      variables: { ...base.variables, ...override.variables },
      output: { ...base.output, ...override.output },
      defaults: { ...base.defaults, ...override.defaults },
    };
  }

  private showHelp(): void {
    console.log(`
${this.logger.color('🚀 CURL RUNNER', 'bright')}

${this.logger.color('USAGE:', 'yellow')}
  curl-runner [files...] [options]

${this.logger.color('OPTIONS:', 'yellow')}
  -h, --help                    Show this help message
  -v, --verbose                 Enable verbose output
  -p, --execution parallel      Execute requests in parallel
  -c, --continue-on-error       Continue execution on errors
  --all                         Find all YAML files recursively
  --output <file>               Save results to file
  --version                     Show version

${this.logger.color('EXAMPLES:', 'yellow')}
  # Run all YAML files in current directory
  curl-runner

  # Run specific file
  curl-runner api-tests.yaml

  # Run all files in a directory
  curl-runner examples/
  
  # Run all files in multiple directories
  curl-runner tests/ examples/

  # Run all files recursively in parallel
  curl-runner --all -p

  # Run directory recursively
  curl-runner --all examples/

  # Run with verbose output and continue on errors
  curl-runner tests/*.yaml -vc

${this.logger.color('YAML STRUCTURE:', 'yellow')}
  Single request:
    request:
      url: https://api.example.com
      method: GET

  Multiple requests:
    requests:
      - url: https://api.example.com/users
        method: GET
      - url: https://api.example.com/posts
        method: POST
        body: { title: "Test" }

  With global config:
    global:
      execution: parallel
      variables:
        BASE_URL: https://api.example.com
    requests:
      - url: \${BASE_URL}/users
        method: GET
`);
  }
}

const cli = new CurlRunnerCLI();
cli.run(process.argv.slice(2));
