/**
 * Simple stdin/stdout test client for the MCP server
 * 
 * This client is designed to be used with an already running MCP server.
 * It reads JSON-RPC responses from stdin and writes requests to stdout.
 * 
 * Usage:
 *   cargo run -- serve | node test/javascript/stdin_client.js
 */
const { stdin, stdout } = process;

// Test sequence
const TEST_SEQUENCE = [
  {
    name: 'initialize',
    request: {
      jsonrpc: '2.0',
      id: 1,
      method: 'initialize',
      params: {}
    }
  },
  {
    name: 'tools/list',
    request: {
      jsonrpc: '2.0',
      id: 2,
      method: 'tools/list',
      params: {}
    }
  },
  {
    name: 'search_code',
    request: {
      jsonrpc: '2.0',
      id: 3,
      method: 'tools/call',
      params: {
        name: 'search_code',
        arguments: {
          query: 'database connection'
        }
      }
    }
  }
];

// Current test index
let currentTestIndex = 0;

// Send the first request
console.error(`Sending ${TEST_SEQUENCE[currentTestIndex].name} request...`);
console.log(JSON.stringify(TEST_SEQUENCE[currentTestIndex].request));

// Buffer for accumulating input
let buffer = '';

// Process stdin data
stdin.on('data', (chunk) => {
  buffer += chunk.toString();
  
  try {
    // Try to parse the buffer as JSON
    const response = JSON.parse(buffer);
    console.error(`Received response for ${TEST_SEQUENCE[currentTestIndex].name}:`);
    console.error(JSON.stringify(response, null, 2));
    
    // Move to the next test
    currentTestIndex++;
    
    // If we have more tests to run, send the next request
    if (currentTestIndex < TEST_SEQUENCE.length) {
      console.error(`\nSending ${TEST_SEQUENCE[currentTestIndex].name} request...`);
      console.log(JSON.stringify(TEST_SEQUENCE[currentTestIndex].request));
    } else {
      console.error('\nAll tests completed!');
      process.exit(0);
    }
    
    // Reset the buffer
    buffer = '';
  } catch (e) {
    // If we can't parse as JSON yet, we need more data
    console.error('Waiting for more data...');
  }
});

// Handle EOF
stdin.on('end', () => {
  console.error('Server closed the connection');
  process.exit(0);
});

// Handle errors
stdin.on('error', (err) => {
  console.error('Error:', err);
  process.exit(1);
});