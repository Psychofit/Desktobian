#!/usr/bin/env python3
"""Desktobian local static file server for web wallpapers.

QtWebEngine's Fetch API cannot load ``file://`` URLs, so web wallpapers that
fetch local assets (e.g. a Rive ``.riv`` file, JSON, shaders) render black when
loaded from ``file://``. Serving the wallpaper over ``http://127.0.0.1`` gives
the page a real HTTP origin, so those fetches — and cross-origin CDN fetches —
work normally.

The server is intentionally tiny and conservative:

* binds to ``127.0.0.1`` only (never reachable from the network),
* serves files read-only over ``GET``/``HEAD`` (no uploads, no directory
  listing),
* exposes only files the current user can already read,
* exits quietly if the port is already taken (so it is safe to start more than
  once, e.g. from autostart and from an installer).

It maps the URL path straight onto the filesystem, so an absolute wallpaper path
``/path/to/index.html`` is reachable at ``http://127.0.0.1:47821/path/to/index.html``.
"""

import http.server
import socketserver
import sys

PORT = 47821
ROOT = "/"


class Handler(http.server.SimpleHTTPRequestHandler):
    def __init__(self, *args, **kwargs):
        super().__init__(*args, directory=ROOT, **kwargs)

    def end_headers(self):
        # Let the wallpaper page fetch its own assets and CDN resources freely.
        self.send_header("Access-Control-Allow-Origin", "*")
        self.send_header("Cache-Control", "no-store")
        super().end_headers()

    def list_directory(self, path):
        self.send_error(403, "Directory listing is disabled")
        return None

    def log_message(self, *args):
        pass  # stay quiet; this runs in the background


class Server(socketserver.ThreadingTCPServer):
    allow_reuse_address = True
    daemon_threads = True


def main():
    try:
        with Server(("127.0.0.1", PORT), Handler) as httpd:
            httpd.serve_forever()
    except OSError as exc:
        # Most likely the port is already in use -> another instance is running.
        sys.stderr.write("desktobian-webserver: %s\n" % exc)
        return 0


if __name__ == "__main__":
    sys.exit(main() or 0)
