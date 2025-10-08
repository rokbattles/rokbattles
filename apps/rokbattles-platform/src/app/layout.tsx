export default function Layout({ children }: LayoutProps<"/">) {
  return (
    <html lang="en">
      <head>
        <link rel="preconnect" href="https://plat-fau-global.lilithgame.com" crossOrigin="" />
        <link rel="dns-prefetch" href="https://imimg.lilithcdn.com" />
        <link rel="dns-prefetch" href="https://imv2-gl.lilithgame.com" />
        <link rel="dns-prefetch" href="https://static-gl.lilithgame.com" />
      </head>
      <body>{children}</body>
    </html>
  );
}
