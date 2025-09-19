import { NextResponse } from 'next/server'

export async function GET() {
  const rss = `<?xml version="1.0" encoding="UTF-8"?>
<rss version="2.0" xmlns:atom="http://www.w3.org/2005/Atom">
  <channel>
    <title>curl-runner Documentation</title>
    <description>A powerful CLI tool for HTTP request management using YAML configuration files. Built with Bun for blazing-fast performance.</description>
    <link>https://www.curl-runner.com</link>
    <language>en-us</language>
    <managingEditor>noreply@curl-runner.com (curl-runner Team)</managingEditor>
    <webMaster>noreply@curl-runner.com (curl-runner Team)</webMaster>
    <lastBuildDate>${new Date().toUTCString()}</lastBuildDate>
    <atom:link href="https://www.curl-runner.com/rss.xml" rel="self" type="application/rss+xml"/>

    <item>
      <title>curl-runner Documentation</title>
      <link>https://www.curl-runner.com/docs</link>
      <description>Complete documentation for curl-runner CLI tool</description>
      <guid>https://www.curl-runner.com/docs</guid>
      <pubDate>${new Date().toUTCString()}</pubDate>
    </item>

    <item>
      <title>Quick Start Guide</title>
      <link>https://www.curl-runner.com/docs/quick-start</link>
      <description>Get started with curl-runner in minutes</description>
      <guid>https://www.curl-runner.com/docs/quick-start</guid>
      <pubDate>${new Date().toUTCString()}</pubDate>
    </item>

    <item>
      <title>Installation Guide</title>
      <link>https://www.curl-runner.com/docs/installation</link>
      <description>Install curl-runner using your preferred package manager</description>
      <guid>https://www.curl-runner.com/docs/installation</guid>
      <pubDate>${new Date().toUTCString()}</pubDate>
    </item>

    <item>
      <title>Examples</title>
      <link>https://www.curl-runner.com/docs/examples/basic</link>
      <description>Learn curl-runner with practical examples</description>
      <guid>https://www.curl-runner.com/docs/examples/basic</guid>
      <pubDate>${new Date().toUTCString()}</pubDate>
    </item>

  </channel>
</rss>`

  return new NextResponse(rss, {
    status: 200,
    headers: {
      'Content-Type': 'application/rss+xml; charset=utf-8',
      'Cache-Control': 'public, max-age=3600, stale-while-revalidate=86400'
    }
  })
}