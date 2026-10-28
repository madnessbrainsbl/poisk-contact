import http.server
import threading
import unittest
from unittest import mock
import scrapling_fetch


class Fixture(http.server.BaseHTTPRequestHandler):
    def do_GET(self):
        body = b'<html><body><div id="root"></div><script>document.getElementById("root").innerHTML="Krasnoyarsk <a href=mailto:office@studio.test>office@studio.test</a>";</script></body></html>'
        self.send_response(200)
        self.send_header("Content-Type", "text/html; charset=utf-8")
        self.end_headers()
        self.wfile.write(body)

    def log_message(self, *args):
        pass


class ScraplingBridgeTest(unittest.TestCase):
    def test_detects_javascript_shell(self):
        self.assertTrue(scrapling_fetch.needs_browser('<div id="root"></div><script src="app.js"></script>'))
        self.assertFalse(scrapling_fetch.needs_browser('<div id="root">office@studio.test</div><script src="app.js"></script>'))
        with self.assertRaises(ValueError):
            scrapling_fetch.validate_url('file:///secret')

    def test_real_browser_executes_javascript(self):
        server = http.server.ThreadingHTTPServer(('127.0.0.1', 0), Fixture)
        worker = threading.Thread(target=server.serve_forever, daemon=True)
        worker.start()
        try:
            url = f'http://127.0.0.1:{server.server_port}/'
            page = scrapling_fetch.render_isolated(url)
            self.assertEqual(page['url'], url)
            self.assertIn('office@studio.test</a>', page['html'])
            self.assertIn('Krasnoyarsk', page['html'])
        finally:
            server.shutdown()
            server.server_close()
            worker.join()


if __name__ == '__main__':
    unittest.main()
