const { execSync } = require('child_process');
const fs = require('fs');

try {
  // Read key exactly as is, trim whitespace/newlines
  const key = fs.readFileSync('../.tauri/top.key', 'utf8').trim();
  
  console.log("Key length:", key.length);
  
  // Set env vars
  process.env.TAURI_SIGNING_PRIVATE_KEY = key;
  process.env.TAURI_SIGNING_PRIVATE_KEY_PASSWORD = "M07@medZakari@";
  
  // Run signer
  console.log("Running tauri signer for 1.0.1...");
  const output = execSync('npm run tauri signer sign -- C:\\Users\\muhammad\\Projects\\Top\\tauri\\src-tauri\\target\\release\\bundle\\nsis\\top_1.0.1_x64-setup.exe', {
    env: process.env,
    stdio: 'inherit'
  });
  
  console.log("Success.");
} catch (e) {
  console.error("Failed:", e.message);
}
