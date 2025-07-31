const { execSync } = require('child_process');
const path = require('path');

console.log('🚀 Running Vercel prebuild steps...');

// Ensure pnpm is installed
console.log('\n🔍 Verifying pnpm installation...');
try {
  execSync('pnpm --version', { stdio: 'inherit' });
  console.log('✅ pnpm is installed');
} catch (error) {
  console.error('❌ pnpm is not installed. Please install pnpm globally: npm install -g pnpm');
  process.exit(1);
}

// Install root dependencies
console.log('\n📦 Installing root dependencies...');
const rootInstallCmd = 'pnpm install --frozen-lockfile --ignore-scripts';
console.log(`Running: ${rootInstallCmd}`);
try {
  execSync(rootInstallCmd, { stdio: 'inherit' });
  console.log('✅ Root dependencies installed successfully');
} catch (error) {
  console.error('❌ Failed to install root dependencies');
  process.exit(1);
}

console.log('\n✨ Prebuild steps completed successfully!');
