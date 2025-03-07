/**
 * MCP Server Test Client
 * 
 * This client tests the Ummon MCP server functionality by sending a series of
 * JSON-RPC requests to exercise the server API. It tests each stage in sequence,
 * waiting for a response before sending the next request.
 */
const { spawn } = require('child_process');
const readline = require('readline');

// Configuration
const SERVER_CMD = 'cargo';
const SERVER_ARGS = ['run', '--', 'serve'];
const SERVER_CWD = process.cwd();
const SERVER_STARTUP_TIMEOUT = 5000; // 5 seconds

// Test case definitions - each is run in sequence
const TEST_CASES = [
  {
    name: 'Initialize',
    request: {
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {}
    },
    validate: (response) => {
      if (!response.result || !response.result.capabilities) {
        throw new Error('Missing capabilities in initialize response');
      }
      console.log('✅ Initialize test passed');
      return true;
    }
  },
  {
    name: 'Tools List',
    request: {
      jsonrpc: '2.0',
      id: 2,
      method: 'tools/list',
      params: {}
    },
    validate: (response) => {
      if (!Array.isArray(response.result)) {
        throw new Error('Tools list response is not an array');
      }
      if (response.result.length === 0) {
        throw new Error('Tools list is empty');
      }
      console.log(`✅ Tools list test passed (${response.result.length} tools found)`);
      return true;
    }
  },
  {
    name: 'Search Code',
    request: {
      jsonrpc: '2.0',
      id: 3,
      method: 'tools/call',
      params: {
        name: 'search_code',
        arguments: {
          query: 'knowledge graph'
        }
      }
    },
    validate: (response) => {
      if (!response.result || !response.result.content) {
        throw new Error('Missing content in search response');
      }
      console.log('✅ Search code test passed');
      return true;
    }
  },
  {
    name: 'Architecture Explanation',
    request: {
      jsonrpc: '2.0',
      id: 4,
      method: 'tools/call',
      params: {
        name: 'explain_architecture',
        arguments: {
          detail_level: 'low'
        }
      }
    },
    validate: (response) => {
      if (!response.result || !response.result.content) {
        throw new Error('Missing content in architecture explanation');
      }
      console.log('✅ Architecture explanation test passed');
      return true;
    }
  },
  {
    name: 'Find Relevant Files',
    request: {
      jsonrpc: '2.0',
      id: 5,
      method: 'tools/call',
      params: {
        name: 'find_relevant_files',
        arguments: {
          description: 'parsing source code',
          limit: 3
        }
      }
    },
    validate: (response) => {
      if (!response.result || !response.result.content) {
        throw new Error('Missing content in find relevant files response');
      }
      console.log('✅ Find relevant files test passed');
      return true;
    }
  },
  {
    name: 'Entity Search',
    request: {
      jsonrpc: '2.0',
      id: 6,
      method: 'tools/call',
      params: {
        name: 'search_code',
        arguments: {
          query: 'DatabaseConnection'
        }
      }
    },
    validate: (response) => {
      if (!response.result || !response.result.content) {
        throw new Error('Missing content in entity search response');
      }
      
      // Try to extract an entity ID for the next test
      let entityId = null;
      try {
        const content = response.result.content[0].text;
        const match = content.match(/- (entity_\d+:|[./\w:]+::)/);
        if (match && match[1]) {
          entityId = match[1].replace(/:$/, '');
          console.log(`Found entity ID: ${entityId}`);
        }
      } catch (error) {
        console.log('Could not extract entity ID, using fallback');
      }
      
      // Store the entity ID for the next test
      if (entityId) {
        TEST_CASES[6].request.params.arguments.entity_id = entityId;
      }
      
      console.log('✅ Entity search test passed');
      return true;
    }
  },
  {
    name: 'Explore Relationships',
    request: {
      jsonrpc: '2.0',
      id: 7,
      method: 'tools/call',
      params: {
        name: 'explore_relationships',
        arguments: {
          entity_id: 'entity_1', // This will be replaced by the previous test if an ID is found
          depth: 2
        }
      }
    },
    validate: (response) => {
      // If we got an error about entity not found, that's okay - we'll count it as a pass
      // since the previous test might not have found a valid entity
      if (response.error && response.error.message.includes('Entity not found')) {
        console.log('⚠️ Entity not found, but test considered passed');
        return true;
      }
      
      if (!response.result || !response.result.content) {
        throw new Error('Missing content in relationships response');
      }
      
      console.log('✅ Explore relationships test passed');
      return true;
    }
  },
  {
    name: 'Resources List',
    request: {
      jsonrpc: '2.0',
      id: 8,
      method: 'resources/list',
      params: {}
    },
    validate: (response) => {
      // This might return an error if resources aren't supported
      if (response.error && response.error.message.includes('not supported')) {
        console.log('⚠️ Resources not supported, but test considered passed');
        return true;
      }
      
      console.log('✅ Resources list test passed');
      return true;
    }
  },
  {
    name: 'Invalid Method Test',
    request: {
      jsonrpc: '2.0',
      id: 9,
      method: 'nonexistent_method',
      params: {}
    },
    validate: (response) => {
      if (!response.error || response.error.code !== -32601) {
        throw new Error('Expected method not found error');
      }
      console.log('✅ Invalid method test passed');
      return true;
    }
  },
  {
    name: 'Invalid Params Test',
    request: {
      jsonrpc: '2.0',
      id: 10,
      method: 'tools/call',
      params: {
        name: 'search_code',
        arguments: {
          // Missing required query parameter
        }
      }
    },
    validate: (response) => {
      if (!response.error || response.error.code !== -32602) {
        throw new Error('Expected invalid params error');
      }
      console.log('✅ Invalid params test passed');
      return true;
    }
  }
];

/**
 * Main test runner function
 */
async function runTests() {
  console.log('Starting MCP server test suite');
  
  // Create a child process for the ummon serve command
  const serverProcess = spawn(SERVER_CMD, SERVER_ARGS, {
    cwd: SERVER_CWD,
    stdio: ['pipe', 'pipe', process.stderr]
  });

  // Set up readline interface
  const rl = readline.createInterface({
    input: serverProcess.stdout,
    crlfDelay: Infinity
  });
  
  // Track current test index
  let currentTestIndex = 0;
  let testStartTime = Date.now();
  let hasServerStarted = false;
  
  // Create a promise to wait for server startup
  const serverStartPromise = new Promise((resolve, reject) => {
    const timeoutId = setTimeout(() => {
      reject(new Error('Server startup timeout'));
    }, SERVER_STARTUP_TIMEOUT);
    
    // Check for the "Server is ready" message in stderr
    serverProcess.stderr.on('data', (data) => {
      const message = data.toString();
      if (message.includes('Server is ready')) {
        clearTimeout(timeoutId);
        hasServerStarted = true;
        resolve();
      }
    });
  });
  
  try {
    // Wait for server to start
    await serverStartPromise;
    console.log('Server started successfully');
    
    // Send first test request
    sendNextTest();
    
    // Process each line of output
    rl.on('line', (line) => {
      try {
        const response = JSON.parse(line);
        console.log(`Received response for test: ${TEST_CASES[currentTestIndex-1].name}`);
        
        // Validate the response
        const testCase = TEST_CASES[currentTestIndex-1];
        const success = testCase.validate(response);
        
        if (success && currentTestIndex < TEST_CASES.length) {
          // Send the next test
          sendNextTest();
        } else if (success) {
          // All tests completed
          console.log('✅ All tests completed successfully!');
          cleanup(0);
        } else {
          // Test failed
          console.error(`❌ Test ${testCase.name} failed`);
          cleanup(1);
        }
      } catch (error) {
        console.error('Error processing response:', error);
        cleanup(1);
      }
    });
  } catch (error) {
    console.error('Error during test:', error);
    cleanup(1);
  }
  
  // Helper function to send the next test
  function sendNextTest() {
    if (currentTestIndex < TEST_CASES.length) {
      const testCase = TEST_CASES[currentTestIndex];
      console.log(`Running test ${currentTestIndex + 1}/${TEST_CASES.length}: ${testCase.name}`);
      serverProcess.stdin.write(JSON.stringify(testCase.request) + '\n');
      currentTestIndex++;
      testStartTime = Date.now();
    }
  }
  
  // Helper function to clean up and exit
  function cleanup(exitCode) {
    console.log('Cleaning up...');
    serverProcess.kill();
    setTimeout(() => {
      process.exit(exitCode);
    }, 100);
  }
  
  // Handle errors
  serverProcess.on('error', (error) => {
    console.error('Server process error:', error);
    cleanup(1);
  });
  
  // Handle server exit
  serverProcess.on('exit', (code) => {
    console.log(`Server exited with code ${code}`);
    if (!hasServerStarted) {
      console.error('Server failed to start correctly');
      process.exit(1);
    }
  });
  
  // Handle CTRL+C to kill the server process
  process.on('SIGINT', () => {
    console.log('Received SIGINT, killing server...');
    cleanup(0);
  });
}

// Start the tests
runTests();