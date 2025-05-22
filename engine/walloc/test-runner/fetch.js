import init, { Walloc } from './wbg/walloc.js';

let allocator = null;
const MB = 1024 * 1024;

async function run() {
  await init();
  allocator = Walloc.new_tiered();

  function log(message) {
    const consoleDiv = document.getElementById('console');
    if (consoleDiv) {
      const line = document.createElement('div');
      line.textContent = message;
      consoleDiv.appendChild(line);
      consoleDiv.scrollTop = consoleDiv.scrollHeight;
    }
    console.log(message);
  }

  function logMemStats() {
    if (allocator) {
      const stats = allocator.memory_stats();
      log(
        `Scene tier: ${(stats.tiers[1].used / 1024).toFixed(2)} KB used, ` +
          `${(stats.tiers[1].highWaterMark / 1024).toFixed(2)} KB high, ` +
          `${(stats.tiers[1].memorySaved / 1024).toFixed(2)} KB saved`
      );
    }
  }

  // Helper to verify asset content
  async function verifyAsset(path, expectedId) {
    try {
      const data = allocator.get_asset(path);
      const jsonData = JSON.parse(new TextDecoder().decode(data));
      const result = jsonData.id === expectedId;
      log(`[${result ? 'PASS' : 'FAIL'}] Asset ${path}: id=${jsonData.id}`);
      return result;
    } catch (error) {
      log(`[FAIL] Error verifying ${path}: ${error.message}`);
      return false;
    }
  }

  try {
    // Test setup
    allocator.set_base_url('https://jsonplaceholder.typicode.com/');
    log('Starting asset tests...');
    logMemStats();

    // Test 1: Load assets
    log('\n1. Loading assets:');
    const offset1 = await allocator.load_asset('todos/1', 1);
    const offset2 = await allocator.load_asset('todos/2', 1);
    const offset3 = await allocator.load_asset('todos/3', 1);
    log(`Assets loaded at offsets: ${offset1}, ${offset2}, ${offset3}`);
    logMemStats();

    // Test 2: Verify content
    log('\n2. Verifying assets:');
    const allValid =
      (await verifyAsset('todos/1', 1)) &&
      (await verifyAsset('todos/2', 2)) &&
      (await verifyAsset('todos/3', 3));
    log(allValid ? '[PASS] All assets verified' : '[FAIL] Verification failed');

    // Test 3: Asset eviction
    log('\n3. Testing eviction:');
    allocator.evict_asset('todos/2');
    log('Asset 2 evicted');
    logMemStats();

    // Verify eviction worked properly
    try {
      allocator.get_asset('todos/2');
      log('[FAIL] Asset 2 should be evicted');
    } catch {
      log('[PASS] Asset 2 properly evicted');
    }

    // Check other assets survived
    const othersValid =
      (await verifyAsset('todos/1', 1)) && (await verifyAsset('todos/3', 3));
    log(
      othersValid
        ? '[PASS] Other assets intact'
        : '[FAIL] Other assets affected'
    );

    // Test 4: Load more and evict all
    log('\n4. Load more and reset:');
    for (let i = 4; i <= 6; i++) {
      await allocator.load_asset(`todos/${i}`, 1);
    }
    log('Added 3 more assets');
    logMemStats();

    // Evict all assets
    for (let i = 1; i <= 6; i++) {
      if (i !== 2) {
        // Skip already evicted asset
        allocator.evict_asset(`todos/${i}`);
      }
    }
    log('All assets evicted');
    logMemStats();

    // Test 5: Recovery and larger asset
    log('\n5. Recovery test with larger asset:');
    await allocator.load_asset('comments', 1);
    log('Loaded larger asset (comments)');
    logMemStats();

    try {
      const commentsData = allocator.get_asset('comments');
      const comments = JSON.parse(new TextDecoder().decode(commentsData));
      log(
        `[PASS] Comments loaded: ${comments.length} items, ` +
          `${(commentsData.length / 1024).toFixed(2)} KB`
      );
    } catch (error) {
      log(`[FAIL] Failed to verify comments: ${error.message}`);
    }

    log('\nTests completed successfully!');
  } catch (error) {
    console.error('Error:', error);
    log(`Fatal error: ${error.message}`);
    document.getElementById('error').textContent = 'Error: ' + error.message;
  }
}

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

updateMemoryDisplay();
run().catch(console.error);
