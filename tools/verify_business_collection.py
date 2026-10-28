"""Re-fetch public source pages and retain evidence for a collection checkpoint."""
import argparse
import concurrent.futures
import csv
import hashlib
import html
import json
import re
import threading
import urllib.parse
import urllib.request
from datetime import datetime, timezone
from html.parser import HTMLParser
from pathlib import Path


class VisibleText(HTMLParser):
    def __init__(self):
        super().__init__()
        self.hidden = 0
        self.parts = []
        self.links = []

    def handle_starttag(self, tag, attrs):
        if tag in ("script", "style", "svg", "noscript"):
            self.hidden += 1
        if tag == "a":
            self.links.extend(value for key, value in attrs if key == "href" and value)

    def handle_endtag(self, tag):
        if tag in ("script", "style", "svg", "noscript"):
            self.hidden = max(0, self.hidden - 1)

    def handle_data(self, data):
        if not self.hidden:
            self.parts.append(data)


def digits(value):
    value = re.sub(r"\D", "", value)
    if len(value) == 10:
        return "7" + value
    return "7" + value[1:] if len(value) == 11 and value.startswith("8") else value


def fetch(url, evidence):
    stamp = datetime.now(timezone.utc).isoformat()
    request = urllib.request.Request(url, headers={"User-Agent": "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 Chrome/122.0.0.0 Safari/537.36"})
    try:
        with urllib.request.urlopen(request, timeout=10) as response:
            raw = response.read(6 * 1024 * 1024)
            charset = response.headers.get_content_charset() or "utf-8"
            body = raw.decode(charset, errors="replace")
            if "\ufffd" in body[:4000] and "windows-1251" in body[:4000].lower():
                body = raw.decode("cp1251", errors="replace")
            key = hashlib.sha256(url.encode() + raw).hexdigest()[:20]
            (evidence / (key + ".html")).write_bytes(raw)
            parser = VisibleText()
            parser.feed(body.replace("--!>", "-->"))
            text = html.unescape(" ".join(parser.parts + parser.links))
            title = re.search(r"(?is)<title[^>]*>(.*?)</title>", body)
            return {"url": url, "final_url": response.url, "status": response.status, "checked_at": stamp,
                    "sha256": hashlib.sha256(raw).hexdigest(), "snapshot": key + ".html",
                    "title": html.unescape(re.sub("<[^>]+>", " ", title[1])).strip() if title else "",
                    "text": text, "body": html.unescape(body)}
    except Exception as error:
        return {"url": url, "checked_at": stamp, "error": str(error)}


def verify(record, evidence, cache, lock):
    hostname = urllib.parse.urlparse(record["url"]).hostname or ""
    listing = any(hostname == name or hostname.endswith("." + name) for name in
                  ("spravker.ru", "doctu.ru", "yp.ru", "sprav.co", "2base.ru", "krspages.ru", "baza-inform.ru", "jsprav.ru", "zoon.ru", "2gis.ru", "flamp.ru", "orgpage.ru", "yell.ru", "spr.ru", "sibdom.ru", "veterinarnye-kliniki.com", "dverprof.com", "infrus.ru", "aion-portent.ru", "poisk-firm.ru", "ouuo.ru", "candy-factory.ru"))
    urls = [record["url"]]
    urls.extend(page["url"] for page in record.get("evidence", []) if page.get("url"))
    urls.extend(re.findall(r"(?:Location evidence|Additional contact source): (https?://[^\s;]+)", record.get("notes", "")))
    pages = []
    for url in dict.fromkeys(urls):
        with lock:
            page = cache.get(url)
        if page is None:
            page = fetch(url, evidence)
            with lock:
                cache[url] = page
        pages.append(page)
    readable = [page for page in pages if "text" in page]
    listing = listing or any(any(term in page.get("title", "").lower() for term in
        ("справочник", "каталог организаций", "каталог компаний", "рейтинг компаний", "список компаний", "business directory", "company directory", "yellow pages")) for page in readable)
    email = record.get("email", "").lower()
    phone = digits(record.get("phone", ""))
    email_ok = bool(email) and any(email in page["text"].lower() or email in page["body"].lower() for page in readable)
    phones = []
    for page in readable:
        phones.extend(digits(match) for match in re.findall(r"(?:\+7|8)[\s()\-\u00a0]*\d{3}[\s()\-\u00a0]*\d{3}[\s\-\u00a0]*\d{2}[\s\-\u00a0]*\d{2}", page["text"]))
        phones.extend(digits(match) for match in re.findall(r"tel:([+\d\s()\-]+)", page["body"], re.I))
        phones.extend(digits(match) for match in re.findall(r"(?<!\d)\(?\d{3}\)?[\s-]+\d{3}[-\s]+\d{2}[-\s]+\d{2}(?!\d)", page["text"]))
    phone_ok = bool(phone) and phone in phones
    city_ok = any("красноярск" in page["text"].lower() or "krasnoyarsk" in page["text"].lower() for page in readable)
    social_ok = any(record.get(field) and record[field].lstrip("@").lower() in page["text"].lower() for field in ("telegram", "vk_url") for page in readable)
    result = {"company": record["company"], "url": record["url"], "email": email, "phone": record.get("phone", ""),
              "email_published": email_ok, "phone_published": phone_ok, "city_published": city_ok, "social_published": social_ok,
              "source_readable": bool(readable), "business_source": not listing,
              "verified": not listing and city_ok and (email_ok or phone_ok or social_ok),
              "pages": [{key: value for key, value in page.items() if key not in ("text", "body")} for page in pages]}
    return result


def main():
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("directory", type=Path)
    parser.add_argument("--retry-unverified", action="store_true", help="Retain successful evidence for unchanged records and retry failed/new records")
    args = parser.parse_args()
    directory = args.directory.resolve()
    evidence = directory / "evidence"
    evidence.mkdir(exist_ok=True)
    records = json.loads((directory / "contacts.json").read_text(encoding="utf-8-sig"))
    previous = {}
    if args.retry_unverified and (directory / "source-checks.json").exists():
        previous = {result["url"]: result for result in json.loads((directory / "source-checks.json").read_text(encoding="utf-8"))["records"]}
    cache = {}
    lock = threading.Lock()
    def check_record(record):
        retained = previous.get(record["url"])
        if retained and retained["verified"] and retained["email"] == record.get("email", "").lower() and digits(retained["phone"]) == digits(record.get("phone", "")):
            snapshots = [page for page in retained["pages"] if page.get("snapshot")]
            if snapshots and all((evidence / page["snapshot"]).exists() and hashlib.sha256((evidence / page["snapshot"]).read_bytes()).hexdigest() == page["sha256"] for page in snapshots):
                retained["company"] = record["company"]
                retained["retained_successful_evidence"] = True
                return retained
        return verify(record, evidence, cache, lock)

    with concurrent.futures.ThreadPoolExecutor(max_workers=6) as pool:
        results = list(pool.map(check_record, records))
    report = {"checked_at": datetime.now(timezone.utc).isoformat(), "total": len(results),
              "verified": sum(result["verified"] for result in results), "records": results}
    serialized = json.dumps(report, ensure_ascii=False, indent=2)
    (directory / "source-checks.json").write_text(serialized, encoding="utf-8")
    (directory / ("source-checks-" + datetime.now(timezone.utc).strftime("%Y%m%dT%H%M%S%f") + ".json")).write_text(serialized, encoding="utf-8")
    with (directory / "business_contacts.csv").open("w", encoding="utf-8-sig", newline="") as stream:
        writer = csv.writer(stream, delimiter=";")
        writer.writerow(["Компания", "Отрасль", "Город", "Email", "Телефон", "Telegram", "VK", "Адрес", "Источник", "Проверка источника"])
        for record, result in zip(records, results):
            writer.writerow([record.get(field, "") for field in ("company", "stack", "city", "email", "phone", "telegram", "vk_url", "address", "url")] + ["Подтверждён" if result["verified"] else "Нужна проверка"])
    print(json.dumps({key: value for key, value in report.items() if key != "records"}, ensure_ascii=False))
    for result in results:
        if not result["verified"]:
            print("REVIEW", result["url"], "city:", result["city_published"], "email:", result["email_published"], "phone:", result["phone_published"])


if __name__ == "__main__":
    main()
