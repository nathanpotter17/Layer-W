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

    // runStartTest();
    runInitTest();
  } catch (error) {
    log(`Error: ${error}`);
  }
}

// Simulate start-up.
function runStartTest() {
  log('Loading new level...');
  allocator.reset_tier(TIER.SCENE);
  const objOffset = allocator.allocate_tiered(10 * MB, TIER.SCENE);
  log(`Loaded 100MB Level in SCENE tier ${objOffset}`);

  log('Renderer Starting Up for 1080 x 720...');
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
    `Allocated 3 textures in SCENE tier: 256KB, 1MB, 4MB, and 8MB. ${smallOffset} ${mediumOffset} ${largeOffset} ${hugeOffset}`
  );

  log('Scene Loaded.');
}

// Simple game loop
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
    log(`Allocated ${renderSize / MB}MB in RENDER tier ${renderOffset}`);
  }

  // 2. Test SCENE tier - allocate 3 different texture sizes every 60 frames - ~5.5MB every 60 frames. Continously grows the needed memory, updating total memory.
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

// Simulate start-up with memory preservation
function runInitTest() {
  log('Loading new scene...');

  // First, make sure scene is reset at startup
  allocator.reset_tier(TIER.SCENE);

  // Simulate loading all the scene data into the main buffer, in one big chunk.
  const persistentDataSize = 74 * MB;
  const persistentDataSizeMB = (persistentDataSize / MB).toFixed(2);
  const largeOffset = allocator.allocate_tiered(persistentDataSize, TIER.SCENE);
  log(
    `Total persistent data allocated: ${persistentDataSizeMB}MB, ${largeOffset}`
  );

  // Initialize render resources
  log('Renderer starting up for 1080p resolution...');
  const renderSize = 3 * MB;
  const renderOffset = allocator.allocate_tiered(renderSize, TIER.RENDER);
  log(
    `Allocated ${renderSize / MB}MB in RENDER tier at offset ${renderOffset}`
  );

  // Store the persistent data size globally for use in frame function
  window.persistentDataSize = persistentDataSize;

  // Get memory stats after initialization
  const initStats = allocator.memory_stats();
  log('Initial memory usage:');
  logMemoryStats(initStats);

  log('Startup complete. Beginning frame loop...');
}

// Helper function to log memory stats in a readable format
function logMemoryStats(stats) {
  const tierNames = ['Render', 'Scene', 'Entity'];

  stats.tiers.forEach((tier, i) => {
    const usedMB = (tier.used / MB).toFixed(2);
    const capacityMB = (tier.capacity / MB).toFixed(2);
    const utilizationPercent = ((tier.used / tier.capacity) * 100).toFixed(2);

    log(
      `${tierNames[i]} tier: ${usedMB}MB/${capacityMB}MB (${utilizationPercent}%)`
    );
  });

  log(`Total memory utilization: ${stats.memoryUtilization.toFixed(2)}%`);
}

// Simulate frame updates with recycling
function runFrameTest() {
  frameCount++;

  const shouldLog =
    frameCount % 30 === 0 || frameCount < 5 || frameCount % 60 === 0;
  if (shouldLog) {
    log(`--- Frame ${frameCount} ---`);
  }

  // 1. Reset RENDER tier every frame (traditional approach)
  allocator.reset_tier(TIER.RENDER);
  if (shouldLog) {
    log('Reset RENDER tier');
  }
  // Allocate one frame at a time
  const renderSize = 3 * MB;
  allocator.allocate_tiered(renderSize, TIER.RENDER);

  // 2. ENTITY tier - particles, effects, etc.
  // Reset entity tier every 10 frames
  if (frameCount % 10 === 0) {
    allocator.reset_tier(TIER.ENTITY);
    if (shouldLog) {
      log('Reset ENTITY tier for new effects');
    }
  }
  // Add some particles (10KB each)
  const particleSize = 10 * 1024;
  for (let i = 0; i < 5; i++) {
    allocator.allocate_tiered(particleSize, TIER.ENTITY);
  }

  // 3. SCENE tier - using fast_compact_tier for recycling
  // Approach: Every 60 frames, we'll recycle non-persistent memory while preserving essential data.

  // Add temporary scene objects every frame, simulating continuous particle or mesh based effects
  const tempObjectSize = 50 * 1024; // 50KB per object
  const numObjects = 2; // Add 2 objects per frame (100KB)

  for (let i = 0; i < numObjects; i++) {
    allocator.allocate_tiered(tempObjectSize, TIER.SCENE);
  }

  // Every 60 frames, use fast_compact_tier to preserve just the persistent data
  if (frameCount % 60 === 0) {
    // Log memory stats before compaction
    if (shouldLog) {
      log('Memory stats before compaction:');
      logMemoryStats(allocator.memory_stats());
    }

    // Test different preservation sizes to verify our improved compaction logic
    // First regular case: preserve exactly what we initially set aside
    let preserveSize = window.persistentDataSize;

    // Every 180 frames, try preserving more than we currently have allocated
    // to test the growth logic in our improved fast_compact_tier
    if (frameCount % 180 === 0) {
      // Try to preserve more than what's currently allocated
      // This would have failed before our improvement, but should now work
      preserveSize = window.persistentDataSize + 20 * MB;
      log(
        `TESTING LARGER PRESERVATION: Trying to preserve ${(
          preserveSize / MB
        ).toFixed(2)}MB`
      );
    }

    // Use fast_compact_tier with our test size
    const recycleResult = allocator.fast_compact_tier(TIER.SCENE, preserveSize);

    if (shouldLog) {
      log(
        `RECYCLED SCENE memory with fast_compact_tier(${(
          preserveSize / MB
        ).toFixed(2)}MB)`
      );
      log(`Compaction result: ${recycleResult ? 'SUCCESS' : 'FAILED'}`);

      // If we were testing the larger preservation, and it worked, update our persistent size
      if (frameCount % 180 === 0 && recycleResult) {
        window.persistentDataSize = preserveSize;
        log(
          `Updated persistent data size to ${(
            window.persistentDataSize / MB
          ).toFixed(2)}MB`
        );
      }

      // Log memory stats after compaction
      log('Memory stats after compaction:');
      logMemoryStats(allocator.memory_stats());
    }

    // After recycling, add some new textures
    // Small texture (1024x1024 RGBA = 4MB)
    const smallTexture = 1024 * 1024 * 4;
    const smallOffset = allocator.allocate_tiered(smallTexture, TIER.SCENE);

    // Medium texture (2048x2048 RGBA = 8MB)
    const mediumTexture = 2048 * 2048 * 4;
    const mediumOffset = allocator.allocate_tiered(mediumTexture, TIER.SCENE);

    // Large texture (4096x4096 RGBA = 16MB)
    const largeTexture = 4096 * 4096 * 4;
    const largeOffset = allocator.allocate_tiered(largeTexture, TIER.SCENE);

    if (shouldLog) {
      log(
        `Allocated 3 new textures in SCENE tier after recycling: offsets ${smallOffset}, ${mediumOffset}, ${largeOffset}`
      );

      // Log memory stats after new allocations
      log('Memory stats after new allocations:');
      logMemoryStats(allocator.memory_stats());
    }
  }

  frameId = requestAnimationFrame(runFrameTest);
}

function startSimulation() {
  log('Starting simulation');
  if (!frameId) {
    frameCount = 0;
    frameId = requestAnimationFrame(runFrameTest);
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
      ).toFixed(3)} MB\nMemory Utilization: ${stats.memoryUtilization.toFixed(
        3
      )}% \nTotal Memory Available: ${(stats.rawMemorySize / MB).toFixed(
        3
      )} MB \n\nTier 1 - Render System \nBytes Used: ${
        stats.tiers[0].used / MB
      } MB \nTotal Capacity: ${(stats.tiers[0].capacity / MB).toFixed(
        3
      )} MB \nHigh Water Mark: ${(stats.tiers[0].highWaterMark / MB).toFixed(
        3
      )} MB\nTotal Allocated: ${(stats.tiers[0].totalAllocated / MB).toFixed(
        3
      )} MB\nMemory Saved: ${((stats.tiers[0].memorySaved || 0) / MB).toFixed(
        3
      )} MB\n\nTier 2 - Scene System \nBytes Used: ${
        stats.tiers[1].used / MB
      } MB \nTotal Capacity: ${(stats.tiers[1].capacity / MB).toFixed(
        3
      )} MB \nHigh Water Mark: ${(stats.tiers[1].highWaterMark / MB).toFixed(
        3
      )} MB\nTotal Allocated: ${(stats.tiers[1].totalAllocated / MB).toFixed(
        3
      )} MB\nMemory Saved: ${((stats.tiers[1].memorySaved || 0) / MB).toFixed(
        3
      )} MB\n\nTier 3 - Entity System \nBytes Used: ${(
        stats.tiers[2].used / MB
      ).toFixed(3)} MB \nTotal Capacity: ${(
        stats.tiers[2].capacity / MB
      ).toFixed(3)} MB \nHigh Water Mark: ${(
        stats.tiers[2].highWaterMark / MB
      ).toFixed(3)} MB\nTotal Allocated: ${(
        stats.tiers[2].totalAllocated / MB
      ).toFixed(3)} MB\nMemory Saved: ${(
        (stats.tiers[2].memorySaved || 0) / MB
      ).toFixed(3)} MB`;
    }
  }
  setTimeout(updateMemoryDisplay, 300);
}

if (typeof document !== 'undefined') {
  document.addEventListener('DOMContentLoaded', updateMemoryDisplay);
}
