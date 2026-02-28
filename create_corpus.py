import struct, os
import json

base = r'H:\Code\Rust\shiplog\fuzz\corpus'

def write_bin(target, filename, data):
    d = os.path.join(base, target)
    os.makedirs(d, exist_ok=True)
    with open(os.path.join(d, filename), 'wb') as f:
        f.write(data)

# cache_stats: 24 bytes = 3x i64 LE (total_entries, expired_entries, cache_size_bytes)
write_bin('cache_stats', 'normal_stats', struct.pack('<qqq', 10, 2, 1048576))
write_bin('cache_stats', 'zero_stats', struct.pack('<qqq', 0, 0, 0))
write_bin('cache_stats', 'large_stats', struct.pack('<qqq', 100, 50, 10485760))

# cache_key: split into 3 parts (query | url | tail with page/per_page/project_id/mr_iid bytes)
write_bin('cache_key', 'valid_search', b'author:octo merged:2025' + b'https://api.github.com/repos/o/r' + b'\x01\x1e\x05\x03')
write_bin('cache_key', 'minimal', b'q' + b'u' + b'\x00\x01\x00\x00')

# cache_sqlite: first byte = action (0-4), then key byte, then rest as value
write_bin('cache_sqlite', 'action_set', b'\x00\x05hello-value')
write_bin('cache_sqlite', 'action_ttl', b'\x01\x03\x80test-ttl')
write_bin('cache_sqlite', 'action_contains', b'\x02\x07check')
write_bin('cache_sqlite', 'action_reopen', b'\x03\x01data')
write_bin('cache_sqlite', 'action_clear', b'\x04\x02clear-data')

# cache_expiry: 32 bytes = base_raw(i64) + ttl_raw(i64) + skew_raw(i64) + 8 bytes for parse
write_bin('cache_expiry', 'valid_base', struct.pack('<qqq', 1735689600, 3600, 0) + b'2025-01-')
write_bin('cache_expiry', 'negative_ttl', struct.pack('<qqq', 1735689600, -3600, 7200) + b'20250101')

# date_windows: 24 bytes = since_raw(i64) + until_raw(i64) + 8 padding bytes
write_bin('date_windows', 'normal_range', struct.pack('<qq', 100, 500) + b'\x00' * 8)
write_bin('date_windows', 'year_range', struct.pack('<qq', 0, 365) + b'\x01' + b'\x00' * 7)
write_bin('date_windows', 'same_date', struct.pack('<qq', 50, 50) + b'\x00' * 8)

# cluster_llm_parse: first byte = event_count, second = workstream_count, then workstream data
write_bin('cluster_llm_parse', 'one_ws_two_events', b'\x02\x01\x02\x00\x01\x00\x01')
write_bin('cluster_llm_parse', 'empty', b'\x00\x00')

# cluster_llm_prompt: event_count, max_tokens byte, max_workstreams byte
write_bin('cluster_llm_prompt', 'small_set', b'\x05\x10\x03')
write_bin('cluster_llm_prompt', 'single_event', b'\x01\x08\x01')
write_bin('cluster_llm_prompt', 'empty', b'\x00\x01\x01')

# receipt_markdown: selector(1) + title + repo + url (split into thirds)
write_bin('receipt_markdown', 'pr_receipt', b'\x00Fix payment flowacme/paymentshttps://github.com/acme/payments/pull/42')
write_bin('receipt_markdown', 'review_receipt', b'\x01CI fixacme/platformhttps://github.com/acme/platform/pull/77')
write_bin('receipt_markdown', 'manual_receipt', b'\x02Incident responseacme/opshttps://wiki.internal/incident-42')

# redaction_repo: selector(1) + full_name + url (split at midpoint)
write_bin('redaction_repo', 'private_repo', b'\x00acme/paymentshttps://github.com/acme/payments')
write_bin('redaction_repo', 'public_repo', b'\x01acme/platformhttps://github.com/acme/platform')

# workstream_receipt_policy: idx(1) + kind_selector(1) + count(1) + cap_len(1)
write_bin('workstream_receipt_policy', 'pr_small', b'\x00\x00\x03\x05')
write_bin('workstream_receipt_policy', 'review_large', b'\x05\x01\x10\x20')
write_bin('workstream_receipt_policy', 'manual_zero', b'\x00\x02\x00\x00')

# redact_event: profile_selector(1) + key(32 bytes) + json event
pr_event = json.dumps({"id":"seed_pr_1","kind":"PullRequest","occurred_at":"2025-01-15T16:00:00Z","actor":{"login":"octo","id":None},"repo":{"full_name":"acme/payments","html_url":"https://github.com/acme/payments","visibility":"Private"},"payload":{"type":"PullRequest","data":{"number":42,"title":"Fix payment flow","state":"Merged","created_at":"2025-01-10T12:00:00Z","merged_at":"2025-01-15T16:00:00Z","additions":10,"deletions":3,"changed_files":2,"touched_paths_hint":[],"window":{"since":"2025-01-01","until":"2025-02-01"}}},"tags":["fix"],"links":[{"label":"pr","url":"https://github.com/acme/payments/pull/42"}],"source":{"system":"github","url":None,"opaque_id":"42"}}, separators=(',',':'))

key = b'my-redaction-key-12345678' + b'\x00' * 7  # 32 bytes
write_bin('redact_event', 'internal_pr', b'\x00' + key + pr_event.encode('utf-8'))
write_bin('redact_event', 'public_pr', b'\x02' + key + pr_event.encode('utf-8'))

print("All binary corpus files created successfully!")
