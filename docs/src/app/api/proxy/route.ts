import { openapi } from '@/lib/source';

export const { GET, HEAD, PUT, POST, PATCH, DELETE } = openapi.createProxy({
  // Allow requests to the funnel server
  allowedOrigins: ['http://localhost:8080', 'https://localhost:8080'],
});
