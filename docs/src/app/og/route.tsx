import { ImageResponse } from 'next/og';
import { NextRequest } from 'next/server';

export const runtime = 'edge';

export async function GET(request: NextRequest) {
  try {
    const { searchParams } = new URL(request.url);
    const title = searchParams.get('title') || 'funnel';
    const description = searchParams.get('description') || 'A tunneling solution built with Go';

    return new ImageResponse(
      (
        <div
          style={{
            height: '100%',
            width: '100%',
            display: 'flex',
            flexDirection: 'column',
            alignItems: 'center',
            justifyContent: 'center',
            backgroundColor: '#000',
            backgroundImage: 'linear-gradient(135deg, #667eea 0%, #764ba2 100%)',
            fontSize: '32px',
            fontWeight: 600,
            color: 'white',
            padding: '40px',
          }}
        >
          <div
            style={{
              display: 'flex',
              alignItems: 'center',
              justifyContent: 'center',
              marginBottom: '40px',
            }}
          >
            <div
              style={{
                fontSize: '80px',
                marginRight: '20px',
              }}
            >
              🕳️
            </div>
            <div
              style={{
                fontSize: '72px',
                fontWeight: 'bold',
              }}
            >
              funnel
            </div>
          </div>
          <div
            style={{
              fontSize: '48px',
              fontWeight: 'bold',
              marginBottom: '20px',
              textAlign: 'center',
              maxWidth: '90%',
            }}
          >
            {title}
          </div>
          <div
            style={{
              fontSize: '24px',
              textAlign: 'center',
              maxWidth: '80%',
              opacity: 0.8,
              lineHeight: 1.4,
            }}
          >
            {description}
          </div>
          <div
            style={{
              position: 'absolute',
              bottom: '40px',
              right: '40px',
              fontSize: '18px',
              opacity: 0.7,
            }}
          >
            funnel.karolbroda.com
          </div>
        </div>
      ),
      {
        width: 1200,
        height: 630,
      }
    );
  } catch (error) {
    console.error('Error generating OG image:', error);
    return new Response('Failed to generate image', { status: 500 });
  }
} 