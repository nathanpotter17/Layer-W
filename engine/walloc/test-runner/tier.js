import init, { Walloc } from './wbg/walloc.js';

let allocator = null;
let frameId = null;
let frameCount = 0;

const MB = 1024 * 1024;
const TIER = { RENDER: 0, SCENE: 1, ENTITY: 2 };

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

    runStartTest();
  } catch (error) {
    log(`Error: ${error}`);
  }
}

// Simulate start-up.
function runStartTest() {
  log('Managing Entities');
  const objOffset = allocator.allocate_tiered(100 * MB, TIER.ENTITY);
  log(`Allocated 100MB in Entity tier ${objOffset}`);

  log('Renderer Starting Up...');
  const renderSize = 3 * MB;
  const renderOffset = allocator.allocate_tiered(renderSize, TIER.RENDER);
  log(`Allocated ${renderSize / MB}MB in RENDER tier ${renderOffset}`);

  // Small texture (256x256 RGBA = 262KB)
  const smallTexture = 256 * 256 * 4;
  const smallOffset = allocator.allocate_tiered(smallTexture, TIER.SCENE);

  // Medium texture (512x512 RGBA = 1MB)
  const mediumTexture = 512 * 512 * 4;
  const mediumOffset = allocator.allocate_tiered(mediumTexture, TIER.SCENE);

  // 1k texture (1024x1024 RGBA = 4MB)
  const largeTexture = 1024 * 1024 * 4;
  const largeOffset = allocator.allocate_tiered(largeTexture, TIER.SCENE);

  // 2k texture - 8MB
  const hugeTexture = 2048 * 2048 * 4;
  const hugeOffset = allocator.allocate_tiered(hugeTexture, TIER.SCENE);

  log(
    `Allocated 3 textures in SCENE tier: 256KB, 1MB, and 4MB. ${smallOffset} ${mediumOffset} ${largeOffset} ${hugeOffset}`
  );

  log('Scene Loaded.');
}

function runTest() {
  frameCount++;

  const shouldLog = frameCount % 10 === 0 || frameCount < 5;
  if (shouldLog) {
    log(`--- Frame ${frameCount} ---`);
  }

  // 1. Test RENDER tier - Allocate Every Frame - 1080 x 720 Frame - 4 channel, 32-bit color depth - ~2.97MB/frame
  // Clear image, this will flip the memory to become available.
  // The bump allocation is very fast - it's essentially just an atomic addition operation to advance a pointer.
  // The underlying code uses atomic operations to manage the offsets, which makes it thread-safe even in concurrent environments.
  // Instead of repeatedly allocating and freeing, which could lead to fragmentation, reset the entire arena at once and start fresh for each frame.
  allocator.reset_tier(TIER.RENDER);
  if (shouldLog) {
    log('Reset RENDER tier');
  }

  // Allocate one frame at a time.
  const renderSize = 3 * MB;
  const renderOffset = allocator.allocate_tiered(renderSize, TIER.RENDER);
  if (shouldLog) {
    log(`Allocated ${renderSize / MB}MB in RENDER tier`, renderOffset);
  }

  // 2. Test SCENE tier - allocate 3 different texture sizes every 60 frames - ~5.5MB every 60 frames
  if (frameCount % 60 === 0) {
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
      log(
        `Allocated 3 textures in SCENE tier: 256KB, 1MB, and 4MB. ${smallOffset} ${mediumOffset} ${largeOffset}`
      );
    }
  }

  frameId = requestAnimationFrame(runTest);
}

function startSimulation() {
  log('Starting simulation');
  if (!frameId) {
    frameCount = 0;
    frameId = requestAnimationFrame(runTest);
  }
}

function stopSimulation() {
  if (frameId) {
    cancelAnimationFrame(frameId);
    frameId = null;
    log('Simulation stopped');
  }
}

initWasm();

function updateMemoryDisplay() {
  if (allocator) {
    const stats = allocator.memory_stats();
    const memoryStatsDisplay = document.getElementById('memory-stats');
    if (memoryStatsDisplay) {
      memoryStatsDisplay.textContent = `Total Memory: ${(
        stats.totalSize / MB
      ).toFixed(
        4
      )} MB\n \nTier 1 - Render System \nUtilization: ${stats.tiers[0].percentage.toFixed(
        2
      )}% \nBytes Used: ${stats.tiers[0].used / MB} MB \nTotal Capacity: ${(
        stats.tiers[0].capacity / MB
      ).toFixed(
        2
      )} MB \n\nTier 2 - Scene System \nUtilization: ${stats.tiers[1].percentage.toFixed(
        2
      )}% \nBytes Used: ${stats.tiers[1].used / MB} MB \nTotal Capacity: ${(
        stats.tiers[1].capacity / MB
      ).toFixed(
        2
      )} MB \n\nTier 3 - Entity System \nUtilization: ${stats.tiers[2].percentage.toFixed(
        2
      )}% \nBytes Used: ${(stats.tiers[2].used / MB).toFixed(
        4
      )} MB \nTotal Capacity: ${(stats.tiers[2].capacity / MB).toFixed(2)} MB`;
    }
  }
  setTimeout(updateMemoryDisplay, 300);
}

if (typeof document !== 'undefined') {
  document.addEventListener('DOMContentLoaded', updateMemoryDisplay);
}
