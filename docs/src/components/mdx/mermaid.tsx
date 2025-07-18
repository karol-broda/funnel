'use client';
import { useTheme } from 'next-themes';
import { useEffect, useState, useId, useRef } from 'react';

export default function Mermaid({ chart }: { chart: string }) {
  const id = useId();
  const [isClient, setIsClient] = useState(false);
  const { resolvedTheme } = useTheme();
  const [error, setError] = useState<string | null>(null);
  const containerRef = useRef<HTMLDivElement>(null);

  // ensure we're on client side
  useEffect(() => {
    setIsClient(true);
  }, []);

  useEffect(() => {
    if (!isClient || !chart || !containerRef.current) {
      return;
    }

    let isMounted = true;

    async function renderChart() {
      try {
        // dynamic import to ensure it only loads on client
        const mermaid = (await import('mermaid')).default;
        
        if (!isMounted) return;

        // define theme configuration
        const themeConfig = {
          theme: resolvedTheme === 'dark' ? 'dark' : 'default',
          themeVariables: resolvedTheme === 'dark' ? {
            // dark theme colors (catppuccin mocha)
            primaryColor: '#cba6f7',
            primaryTextColor: '#cdd6f4',
            primaryBorderColor: '#cba6f7',
            lineColor: '#6c7086',
            secondaryColor: '#313244',
            tertiaryColor: '#45475a',
            background: '#1e1e2e',
            mainBkg: '#313244',
            secondBkg: '#313244',
            tertiaryBkg: '#45475a',
            textColor: '#cdd6f4',
            nodeTextColor: '#cdd6f4',
            nodeBkg: '#313244',
            nodeBorder: '#cba6f7',
            clusterBkg: '#181825',
            clusterBorder: '#45475a',
            defaultLinkColor: '#6c7086',
            titleColor: '#cba6f7',
            edgeLabelBackground: '#1e1e2e',
            actorBkg: '#313244',
            actorBorder: '#cba6f7',
            actorTextColor: '#cdd6f4',
            actorLineColor: '#6c7086',
            signalColor: '#cdd6f4',
            signalTextColor: '#cdd6f4',
            labelBoxBkgColor: '#313244',
            labelBoxBorderColor: '#cba6f7',
            labelTextColor: '#cdd6f4',
            loopTextColor: '#89dceb',
            activationBorderColor: '#f2cdcd',
            activationBkgColor: '#45475a',
            sequenceNumberColor: '#a6adc8',
          } : {
            // light theme colors (catppuccin latte)
            primaryColor: '#8839ef',
            primaryTextColor: '#4c4f69',
            primaryBorderColor: '#8839ef',
            lineColor: '#9ca0b0',
            secondaryColor: '#ccd0da',
            tertiaryColor: '#bcc0cc',
            background: '#eff1f5',
            mainBkg: '#ccd0da',
            secondBkg: '#ccd0da',
            tertiaryBkg: '#bcc0cc',
            textColor: '#4c4f69',
            nodeTextColor: '#4c4f69',
            nodeBkg: '#ccd0da',
            nodeBorder: '#8839ef',
            clusterBkg: '#e6e9ef',
            clusterBorder: '#bcc0cc',
            defaultLinkColor: '#9ca0b0',
            titleColor: '#8839ef',
            edgeLabelBackground: '#eff1f5',
            actorBkg: '#ccd0da',
            actorBorder: '#8839ef',
            actorTextColor: '#4c4f69',
            actorLineColor: '#9ca0b0',
            signalColor: '#4c4f69',
            signalTextColor: '#4c4f69',
            labelBoxBkgColor: '#ccd0da',
            labelBoxBorderColor: '#8839ef',
            labelTextColor: '#4c4f69',
            loopTextColor: '#04a5e5',
            activationBorderColor: '#d20f39',
            activationBkgColor: '#bcc0cc',
            sequenceNumberColor: '#6c6f85',
          },
          flowchart: {
            curve: 'basis',
            padding: 15,
            useMaxWidth: true,
            htmlLabels: true
          },
          startOnLoad: false,
          securityLevel: 'loose',
        };

        // initialize mermaid
        mermaid.initialize(themeConfig);

        // clear the container
        if (containerRef.current) {
          containerRef.current.innerHTML = '';
        }

        // create a div for mermaid to render into
        const graphDiv = document.createElement('div');
        graphDiv.textContent = chart;
        graphDiv.className = 'mermaid';
        
        // append to our container
        if (containerRef.current) {
          containerRef.current.appendChild(graphDiv);
        }

        // use mermaid.run() which is the newer API
        await mermaid.run({
          nodes: [graphDiv],
        });

        setError(null);
      } catch (err) {
        if (!isMounted) return;
        
        console.error('Error rendering Mermaid chart:', err);
        setError(err instanceof Error ? err.message : 'Failed to render chart');
        
        // clear container on error
        if (containerRef.current) {
          containerRef.current.innerHTML = '';
        }
      }
    }

    // small delay to ensure DOM is ready
    const timeoutId = setTimeout(() => {
      void renderChart();
    }, 0);

    return () => {
      isMounted = false;
      clearTimeout(timeoutId);
    };
  }, [chart, isClient, resolvedTheme]);

  // don't render anything on server
  if (!isClient) {
    return (
      <div
        ref={containerRef}
        style={{
          minHeight: '200px',
          display: 'flex',
          alignItems: 'center',
          justifyContent: 'center'
        }}
      >
        <div>Loading diagram...</div>
      </div>
    );
  }

  // error state
  if (error) {
    return (
      <div
        ref={containerRef}
        style={{
          display: 'flex',
          justifyContent: 'center',
          alignItems: 'center',
          minHeight: '200px',
          color: '#f38ba8',
          padding: '1rem',
          textAlign: 'center',
          fontFamily: 'inherit'
        }}
      >
        Error: {error}
      </div>
    );
  }

  // render container
  return (
    <div 
      ref={containerRef}
      className="mermaid-container"
      style={{
        width: '100%',
        overflow: 'auto',
        fontFamily: 'inherit',
        minHeight: '200px'
      }}
    />
  );
}