import init, { Walloc } from './wbg/walloc.js';

let allocator = null;
let frameId = null;
let frameCount = 0;

const MB = 1024 * 1024;
const TIER = { RENDER: 0, SCENE: 1, ENTITY: 2 };

// Simple logging function
function log(message) {
  console.log(message);
  const consoleDiv = document.getElementById('console');
  if (consoleDiv) {
    const line = document.createElement('div');
    line.textContent = message;
    consoleDiv.appendChild(line);
    consoleDiv.scrollTop = consoleDiv.scrollHeight;
  }
}

// Initialize WebAssembly
async function initWasm() {
  try {
    await init();
    log('WebAssembly module initialized');

    allocator = Walloc.new_tiered();
    log('Walloc tiered allocator created');

    // Set up button event handlers
    const startBtn = document.getElementById('start-btn');
    const stopBtn = document.getElementById('stop-btn');

    if (startBtn) startBtn.addEventListener('click', startSimulation);
    if (stopBtn) stopBtn.addEventListener('click', stopSimulation);

    log('Ready! Click "Start Simulation" to begin');
  } catch (error) {
    log(`Error: ${error}`);
  }
}

// Display tier stats
function showTierStats() {
  const stats = allocator.memory_stats();
  log('Memory stats:');

  if (stats.tiers) {
    stats.tiers.forEach((tier) => {
      log(
        `${tier.name} tier: ${(tier.used / 1024).toFixed(2)}KB / ${(
          tier.capacity / 1024
        ).toFixed(2)}KB (${tier.percentage.toFixed(2)}%)`
      );
    });
  }
}

// Simple test function that runs every frame
function runTest() {
  frameCount++;

  // Only log every 10 frames to avoid spam
  const shouldLog = frameCount % 10 === 0 || frameCount < 5;
  if (shouldLog) {
    log(`--- Frame ${frameCount} ---`);
  }

  // 1. Test RENDER tier - allocate a 1MB block every 30 frames
  if (frameCount % 30 === 0) {
    const renderSize = 1 * MB;
    const renderOffset = allocator.allocate_tiered(renderSize, TIER.RENDER);
    if (shouldLog) {
      log(`Allocated ${renderSize / MB}MB in RENDER tier`);
    }
  }

  // 2. Test SCENE tier - allocate 3 different texture sizes
  if (frameCount % 20 === 0) {
    // Small texture (256x256 RGBA = 262KB)
    const smallTexture = 256 * 256 * 4;
    const smallOffset = allocator.allocate_tiered(smallTexture, TIER.SCENE);

    // Medium texture (512x512 RGBA = 1MB)
    const mediumTexture = 512 * 512 * 4;
    const mediumOffset = allocator.allocate_tiered(mediumTexture, TIER.SCENE);

    // Large texture (1024x1024 RGBA = 4MB)
    const largeTexture = 1024 * 1024 * 4;
    const largeOffset = allocator.allocate_tiered(largeTexture, TIER.SCENE);

    if (shouldLog) {
      log(`Allocated 3 textures in SCENE tier: 256KB, 1MB, and 4MB`);
    }
  }

  // 3. Test ENTITY tier - reset every frame and allocate entities
  allocator.reset_tier(TIER.ENTITY);

  // Allocate 100 entities of 256 bytes each
  for (let i = 0; i < 100; i++) {
    const entityOffset = allocator.allocate_tiered(256, TIER.ENTITY);
  }

  // 4. Reset tiers occasionally
  if (frameCount % 100 === 0) {
    allocator.reset_tier(TIER.SCENE);
    if (shouldLog) {
      log('Reset SCENE tier');
    }
  }

  if (frameCount % 300 === 0) {
    allocator.reset_tier(TIER.RENDER);
    if (shouldLog) {
      log('Reset RENDER tier');
    }
  }

  // 5. Show stats
  if (shouldLog) {
    showTierStats();
  }

  // Continue the loop
  frameId = requestAnimationFrame(runTest);
}

// Start the simulation
function startSimulation() {
  log('Starting simulation');
  if (!frameId) {
    frameCount = 0;
    frameId = requestAnimationFrame(runTest);
  }
}

// Stop the simulation
function stopSimulation() {
  if (frameId) {
    cancelAnimationFrame(frameId);
    frameId = null;
    log('Simulation stopped');
  }
}

// Initialize everything
initWasm();

// Update memory stats display
function updateMemoryDisplay() {
  if (allocator) {
    const stats = allocator.memory_stats();
    const memoryStatsDisplay = document.getElementById('memory-stats');
    if (memoryStatsDisplay) {
      memoryStatsDisplay.textContent = `Total Memory: ${(
        stats.totalSize / MB
      ).toFixed(2)} MB`;
    }
  }
  setTimeout(updateMemoryDisplay, 1000);
}

// Start updating the memory display when the DOM is loaded
if (typeof document !== 'undefined') {
  document.addEventListener('DOMContentLoaded', updateMemoryDisplay);
}
