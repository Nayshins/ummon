// Simple test client for the MCP server
const { spawn } = require('child_process');
const readline = require('readline');

// Create a child process for the ummon serve command
const serverProcess = spawn('cargo', ['run', '--', 'serve'], {
  cwd: '/Users/jnations/dev/ummon',
  stdio: ['pipe', 'pipe', process.stderr]
});

// Send initialize request
const initializeRequest = {
  jsonrpc: '2.0',
  id: 1,
  method: 'initialize',
  params: {}
};

// Write the request to the server's stdin
serverProcess.stdin.write(JSON.stringify(initializeRequest) + '\n');

// Parse the server's stdout line by line
const rl = readline.createInterface({
  input: serverProcess.stdout,
  crlfDelay: Infinity
});

// Process each line of output
rl.on('line', (line) => {
  try {
    const response = JSON.parse(line);
    console.log('Received response:');
    console.log(JSON.stringify(response, null, 2));
    
    // After initialization, send a tools/list request
    if (response.result && response.id === 1) {
      const toolsListRequest = {
        jsonrpc: '2.0',
        id: 2,
        method: 'tools/list',
        params: {}
      };
      
      console.log('Sending tools/list request...');
      serverProcess.stdin.write(JSON.stringify(toolsListRequest) + '\n');
    }
    
    // After getting tools list, try calling the search_code tool
    if (response.result && response.id === 2) {
      const searchRequest = {
        jsonrpc: '2.0',
        id: 3,
        method: 'tools/call',
        params: {
          name: 'search_code',
          arguments: {
            query: 'knowledge graph'
          }
        }
      };
      
      console.log('Sending search_code request...');
      serverProcess.stdin.write(JSON.stringify(searchRequest) + '\n');
    }
    
    // After search, try explain_architecture tool
    if (response.result && response.id === 3) {
      const architectureRequest = {
        jsonrpc: '2.0',
        id: 4,
        method: 'tools/call',
        params: {
          name: 'explain_architecture',
          arguments: {
            detail_level: 'low'
          }
        }
      };
      
      console.log('Sending explain_architecture request...');
      serverProcess.stdin.write(JSON.stringify(architectureRequest) + '\n');
    }
    
    // After architecture explanation, try find_relevant_files tool
    if (response.result && response.id === 4) {
      const filesRequest = {
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
      };
      
      console.log('Sending find_relevant_files request...');
      serverProcess.stdin.write(JSON.stringify(filesRequest) + '\n');
    }
    
    // After find_relevant_files, try explore_relationships tool
    if (response.result && response.id === 5) {
      // First we'll need to get an entity ID to explore
      // For the test, we'll search for an entity related to the knowledge graph
      const entitySearchRequest = {
        jsonrpc: '2.0',
        id: 6,
        method: 'tools/call',
        params: {
          name: 'search_code',
          arguments: {
            query: 'KnowledgeGraph'
          }
        }
      };
      
      console.log('Sending entity search request...');
      serverProcess.stdin.write(JSON.stringify(entitySearchRequest) + '\n');
    }
    
    // After entity search, get the first entity ID and explore its relationships
    if (response.result && response.id === 6) {
      // Extract the first entity ID from the search results
      // This is a bit hacky for test purposes, but works for our demo
      let entityId = 'entity_1'; // Fallback ID
      
      try {
        // Try to extract an entity ID from the content
        const content = response.result.content[0].text;
        const match = content.match(/- (entity_\d+):/);
        if (match && match[1]) {
          entityId = match[1];
        }
      } catch (error) {
        console.log('Could not extract entity ID, using fallback');
      }
      
      const relationshipsRequest = {
        jsonrpc: '2.0',
        id: 7,
        method: 'tools/call',
        params: {
          name: 'explore_relationships',
          arguments: {
            entity_id: entityId,
            depth: 2
          }
        }
      };
      
      console.log('Sending explore_relationships request for entity:', entityId);
      serverProcess.stdin.write(JSON.stringify(relationshipsRequest) + '\n');
    }
    
    // After relationships, try resources/list
    if (response.result && response.id === 7) {
      const resourcesRequest = {
        jsonrpc: '2.0',
        id: 8,
        method: 'resources/list',
        params: {}
      };
      
      console.log('Sending resources/list request...');
      serverProcess.stdin.write(JSON.stringify(resourcesRequest) + '\n');
    }
    
    // After getting resources list, exit the test
    if (response.id === 8) {
      console.log('Test complete, exiting...');
      serverProcess.kill();
      process.exit(0);
    }
  } catch (error) {
    console.error('Error parsing response:', error);
    serverProcess.kill();
    process.exit(1);
  }
});

// Handle errors
serverProcess.on('error', (error) => {
  console.error('Error:', error);
  process.exit(1);
});

// Handle server exit
serverProcess.on('exit', (code) => {
  console.log(`Server exited with code ${code}`);
  process.exit(code);
});

// Handle CTRL+C to kill the server process
process.on('SIGINT', () => {
  console.log('Received SIGINT, killing server...');
  serverProcess.kill();
  process.exit(0);
});