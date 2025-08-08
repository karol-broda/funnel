import { generateFiles } from 'fumadocs-openapi';

void generateFiles({
  // OpenAPI schema file - use the one we downloaded from server
  input: ['./openapi.json'],
  // Output directory for generated MDX files
  output: './content/docs/reference/server-api',
  // Include endpoint descriptions in MDX
  includeDescription: true,
  // Generate per operation (each endpoint gets its own file)
  per: 'operation',
  // Group operations by tag for better organization
  groupBy: 'tag',
});

console.log('✅ OpenAPI documentation generated successfully!');
console.log('📁 Files created in: ./content/docs/reference/server-api/');
console.log('🔗 Make sure your funnel server is running on http://localhost:8080');
