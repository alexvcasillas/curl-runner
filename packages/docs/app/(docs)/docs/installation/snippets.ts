// Installation page snippets
export const npmInstallExample = `npm install -g @curl-runner/cli`;

export const yarnInstallExample = `yarn global add @curl-runner/cli`;

export const pnpmInstallExample = `pnpm install -g @curl-runner/cli`;

export const bunInstallExample = `bun install -g @curl-runner/cli`;

export const npxUsageExample = `npx @curl-runner/cli my-tests.yaml`;

export const verifyInstallationExample = `curl-runner --version`;

export const updateGlobalExample = `# Update via npm
npm update -g @curl-runner/cli

# Update via yarn
yarn global upgrade @curl-runner/cli

# Update via pnpm
pnpm update -g @curl-runner/cli

# Update via bun
bun update -g @curl-runner/cli`;

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

export const linuxBinaryInstallExample = `# Download the binary (replace VERSION with actual version)
curl -LO https://github.com/alexvcasillas/curl-runner/releases/latest/download/curl-runner-cli-VERSION-linux-x64.tar.gz

# Extract the archive
tar -xzf curl-runner-cli-VERSION-linux-x64.tar.gz

# Make executable
chmod +x curl-runner

# Move to PATH (optional)
sudo mv curl-runner /usr/local/bin/

# Verify installation
curl-runner --version`;

export const macOSBinaryInstallExample = `# Download the binary (replace VERSION with actual version)
curl -LO https://github.com/alexvcasillas/curl-runner/releases/latest/download/curl-runner-cli-VERSION-darwin-arm64.tar.gz

# Extract the archive
tar -xzf curl-runner-cli-VERSION-darwin-arm64.tar.gz

# Make executable
chmod +x curl-runner

# Move to PATH (optional)
sudo mv curl-runner /usr/local/bin/

# Verify installation
curl-runner --version`;

export const windowsBinaryInstallExample = `# Download the binary (replace VERSION with actual version)
Invoke-WebRequest -Uri "https://github.com/alexvcasillas/curl-runner/releases/latest/download/curl-runner-cli-VERSION-windows-x64.zip" -OutFile "curl-runner.zip"

# Extract the archive
Expand-Archive -Path curl-runner.zip -DestinationPath .

# Add to PATH (optional - run as Administrator)
$env:PATH += ";$PWD"

# Verify installation
curl-runner --version`;

export const verifyChecksumLinuxExample = `# Download checksums file
curl -LO https://github.com/alexvcasillas/curl-runner/releases/latest/download/SHA256SUMS.txt

# Verify the download
sha256sum -c SHA256SUMS.txt --ignore-missing`;

export const verifyChecksumMacOSExample = `# Download checksums file
curl -LO https://github.com/alexvcasillas/curl-runner/releases/latest/download/SHA256SUMS.txt

# Verify the download
shasum -a 256 -c SHA256SUMS.txt`;

export const verifyChecksumWindowsExample = `# Download checksums file
Invoke-WebRequest -Uri "https://github.com/alexvcasillas/curl-runner/releases/latest/download/SHA256SUMS.txt" -OutFile "SHA256SUMS.txt"

# Get file hash
$hash = (Get-FileHash -Algorithm SHA256 curl-runner-cli-VERSION-windows-x64.zip).Hash

# Compare with expected hash from SHA256SUMS.txt
# (Manually compare the hash output)`;
