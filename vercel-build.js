const { execSync } = require('child_process');
const path = require('path');
const fs = require('fs');

console.log('🚀 Starting Vercel build process with pnpm...');

function runCommand(command, cwd) {
  console.log(`\n📝 Running: ${command} in ${cwd}`);
  try {
    execSync(command, {
      cwd: path.join(__dirname, cwd),
      stdio: 'inherit',
      env: {
        ...process.env,
        NODE_ENV: 'production',
        NPM_CONFIG_PREFER_OFFLINE: 'true',
        NPM_CONFIG_NETWORK_CONCURRENCY: '1',
      },
    });
    return true;
  } catch (error) {
    console.error(`❌ Command failed: ${command}`, error);
    process.exit(1);
  }
}

// Install root dependencies
console.log('\n🔧 Installing root dependencies...');
runCommand('pnpm install --frozen-lockfile --ignore-scripts', '.');

// Install and build backend
console.log('\n🔧 Setting up backend...');
runCommand('pnpm install --frozen-lockfile --ignore-scripts', 'apps/backend');
runCommand('pnpm run build', 'apps/backend');

// Install and build frontend
console.log('\n🎨 Setting up frontend...');
runCommand('pnpm install --frozen-lockfile --ignore-scripts', 'apps/frontend');
runCommand('pnpm run build', 'apps/frontend');

console.log('\n✅ Build process completed successfully!');
