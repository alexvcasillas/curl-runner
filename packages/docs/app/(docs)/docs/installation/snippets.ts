// Installation page snippets
export const npmInstallExample = `npm install -g curl-runner`;

export const yarnInstallExample = `yarn global add curl-runner`;

export const pnpmInstallExample = `pnpm install -g curl-runner`;

export const bunInstallExample = `bun install -g curl-runner`;

export const npxUsageExample = `npx curl-runner my-tests.yaml`;

export const verifyInstallationExample = `curl-runner --version`;

export const updateGlobalExample = `# Update via npm
npm update -g curl-runner

# Update via yarn
yarn global upgrade curl-runner

# Update via pnpm
pnpm update -g curl-runner

# Update via bun
bun update -g curl-runner`;

export const dockerInstallExample = `# Pull the Docker image
docker pull curlrunner/curl-runner:latest

# Run tests using Docker
docker run --rm -v "$(pwd)":/workspace curlrunner/curl-runner:latest my-tests.yaml`;

export const homebrewInstallExample = `# Add the tap
brew tap curl-runner/tap

# Install curl-runner
brew install curl-runner`;

export const chocolateyInstallExample = `# Install via Chocolatey (Windows)
choco install curl-runner`;

export const snapInstallExample = `# Install via Snap (Linux)
sudo snap install curl-runner`;
