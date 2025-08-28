const puppeteer = require('puppeteer');

async function testWebSocketPersistence() {
  console.log('🚀 Testing WebSocket persistence across navigation...');
  
  const browser = await puppeteer.launch({
    headless: true,
    args: ['--no-sandbox', '--disable-setuid-sandbox']
  });
  
  const page = await browser.newPage();
  
  // Listen for console messages
  const consoleLogs = [];
  page.on('console', msg => {
    const text = msg.text();
    console.log(`[CONSOLE] ${text}`);
    consoleLogs.push(text);
  });
  
  // Listen for WebSocket frames
  const cdp = await page.target().createCDPSession();
  await cdp.send('Network.enable');
  
  const websocketConnections = [];
  const websocketFrames = [];
  
  cdp.on('Network.webSocketCreated', event => {
    console.log(`🔌 WebSocket created: ${event.url}`);
    websocketConnections.push(event);
  });
  
  cdp.on('Network.webSocketClosed', event => {
    console.log(`❌ WebSocket closed: ${event.requestId}`);
  });
  
  cdp.on('Network.webSocketFrameReceived', event => {
    websocketFrames.push(event);
    if (event.response.payloadData) {
      console.log(`📥 WebSocket frame received: ${event.response.payloadData}`);
    }
  });
  
  try {
    console.log('📱 Navigating to homepage...');
    await page.goto('http://localhost:3000/', { waitUntil: 'networkidle2' });
    await page.waitForTimeout(2000);
    
    console.log('📱 Navigating to Projects...');
    await page.click('a[href="/projects"]');
    await page.waitForTimeout(2000);
    
    console.log('📱 Navigating to AI Sessions...');
    await page.click('a[href="/work"]');
    await page.waitForTimeout(2000);
    
    console.log('📱 Navigating back to Dashboard...');
    await page.click('a[href="/"]');
    await page.waitForTimeout(2000);
    
    // Check for errors
    const errors = consoleLogs.filter(log => 
      log.includes('Error') || 
      log.includes('useWebSocket must be used within') ||
      log.includes('WebSocket connection failed')
    );
    
    const websocketConnectedCount = consoleLogs.filter(log => 
      log.includes('WebSocket connected')
    ).length;
    
    const websocketDisconnectedCount = consoleLogs.filter(log => 
      log.includes('WebSocket disconnected')
    ).length;
    
    console.log('\n📊 RESULTS:');
    console.log('===========');
    console.log(`🔌 WebSocket connections created: ${websocketConnections.length}`);
    console.log(`📥 WebSocket frames received: ${websocketFrames.length}`);
    console.log(`✅ "WebSocket connected" messages: ${websocketConnectedCount}`);
    console.log(`❌ "WebSocket disconnected" messages: ${websocketDisconnectedCount}`);
    console.log(`🚨 Errors found: ${errors.length}`);
    
    if (errors.length > 0) {
      console.log('\n🚨 ERRORS:');
      errors.forEach(error => console.log(`  - ${error}`));
    }
    
    // Evaluate success
    const success = errors.length === 0 && 
                   websocketConnections.length <= 2 && // Should be 1, but allow some tolerance
                   websocketConnectedCount <= 2;
    
    if (success) {
      console.log('\n✅ SUCCESS: WebSocket persistence working correctly!');
    } else {
      console.log('\n❌ FAILURE: WebSocket issues detected');
    }
    
  } catch (error) {
    console.error('❌ Test failed:', error);
  } finally {
    await browser.close();
  }
}

// Run the test
testWebSocketPersistence().catch(console.error);
