# 📖 Tutorial: Solana DeFi Guardian Execution Guide

Tutorial langkah demi langkah untuk menjalankan audit keamanan token Solana, mengonfigurasi agent ZeroClaw, dan mengeksekusi alur Standard Operating Procedure (SOP) dengan pertahanan Human-in-the-Loop (HITL).

---

## 📋 Prasyarat Sistem (Prerequisites)

1. **Rust Toolchain**: `rustc` dan `cargo` dengan target WASM `wasm32-wasip2`
   ```bash
   rustup target add wasm32-wasip2
   ```
2. **Python 3**: Untuk pengolahan JSON & pembersihan database sederhana.
3. **Repository Codebase**:
   - Repository Plugin Ini: `/home/hengkerprotzy/coding/zeroclaw-solana-plugins`
   - Repository Host ZeroClaw: `/home/hengkerprotzy/coding/zeroclaw`

---

## 🚀 Tahap 1: Pengujian Unit Test Core & WASM Binaries

Sebelum menjalankan daemon, pastikan seluruh 49 unit test keamanan lulus 100%:

```bash
# 1. Masuk ke direktori plugin
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins

# 2. Jalankan unit test untuk ketiga crate
(cd plugins/solana-lite && cargo test)
(cd plugins/token-risk-check && cargo test)
(cd plugins/spl-transfer-build && cargo test)

# 3. Build WASM binaries (target wasm32-wasip2)
(cd plugins/token-risk-check && cargo build --target wasm32-wasip2 --release)
(cd plugins/spl-transfer-build && cargo build --target wasm32-wasip2 --release)
```

---

## 🛡️ Tahap 2: Live Audit via CLI (Read-Only Risk Check)

Anda dapat mengaudit token apa pun secara langsung di mainnet Solana tanpa perlu menjalankan daemon ZeroClaw:

```bash
# Contoh audit mint USDC di Mainnet Solana
(cd plugins/token-risk-check && cargo run --bin token-risk-check-cli EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
```

---

## ⚙️ Tahap 3: Mempersiapkan & Menjalankan ZeroClaw Daemon

```bash
# 1. Bersihkan stale lock pada database (jika ada sisa run lama yang belum selesai)
python3 -c "
import sqlite3, os
db_path = os.path.expanduser('~/.zeroclaw/data/sop/runs.db')
if os.path.exists(db_path):
    conn = sqlite3.connect(db_path)
    c = conn.cursor()
    c.execute('DELETE FROM sop_claims')
    c.execute('UPDATE sop_runs SET terminal = 1 WHERE terminal = 0')
    conn.commit()
    print('✅ Database disiapkan dan disinkronkan.')
"

# 2. Pastikan daemon lama dihentikan
pkill -9 zeroclaw 2>/dev/null || true
sleep 1

# 3. Jalankan Daemon ZeroClaw di Background
(cd /home/hengkerprotzy/coding/zeroclaw && ./target/release/zeroclaw daemon -v > /home/hengkerprotzy/coding/zeroclaw-solana-plugins/daemon_live.log 2>&1 &)

# 4. Verifikasi status kesehatan daemon
sleep 3
curl -s http://127.0.0.1:42617/health | python3 -c "import sys,json; d=json.load(sys.stdin); print('Status Daemon:', d['status'], '| PID:', d['runtime']['pid'])"
```

---

## 🔄 Tahap 4: Eksekusi SOP End-to-End (`solana-transfer-guard`)

SOP ini memiliki 4 langkah dengan 2 checkpoint persetujuan manusia (Human-in-the-Loop):

### Langkah 4.1: Trigger SOP Run Baru via HTTP Gateway
```bash
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins

RESULT=$(curl -s -X POST http://127.0.0.1:42617/api/sops/solana-transfer-guard/run \
  -H "Content-Type: application/json" \
  -d '{"payload": "{\"mint_address\": \"So11111111111111111111111111111111111111112\", \"from\": \"4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU\", \"to\": \"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8\", \"amount\": \"100000\"}"}')

echo "Respon Trigger: $RESULT"

# Simpan RUN_ID ke variabel terminal
export RUN_ID=$(echo $RESULT | python3 -c "import sys,json; print(json.load(sys.stdin).get('run_id',''))")
echo "RUN_ID Aktif: $RUN_ID"
```

### Langkah 4.2: Cek Status Overlay & Approve HITL Gate Pertama (Step 2)
```bash
# 1. Cek status overlay (Step 1 selesai, Step 2 waiting_approval)
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool

# 2. Berikan persetujuan manusia untuk lanjut ke Step 3 (Build Tx)
/home/hengkerprotzy/coding/zeroclaw/target/release/zeroclaw sop approve $RUN_ID
```

### Langkah 4.3: Polling & Approve HITL Gate Kedua (Step 4)
```bash
# Tunggu ~8-10 detik sampai Step 3 selesai membangun unsigned transaction payload
sleep 8

# Cek overlay status (Step 4 waiting_approval)
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool

# Berikan persetujuan final manusia untuk menyelesaikan SOP
/home/hengkerprotzy/coding/zeroclaw/target/release/zeroclaw sop approve $RUN_ID
```

### Langkah 4.4: Verifikasi Hasil Akhir SOP (`completed`)
```bash
sleep 3

# Cek overlay akhir: Harus menunjukkan status 'completed' pada ke-4 node
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool
```

---

## 🔍 Tahap 5: Pengujian Pertahanan Prompt Injection

Uji ketahanan plugin terhadap upaya instruksi manipulatif (seperti transfer `"all"` atau `"max"`):

```bash
# Jalankan test khusus fail-closed di level Rust WASM
(cd plugins/spl-transfer-build && cargo test injection_via_amount_field_fails_closed -- --nocapture)
(cd plugins/token-risk-check && cargo test prompt_injection_in_mint_address_fails_closed -- --nocapture)
```

---

## 🛠️ Troubleshooting Ringkas

| Gejala Error | Penyebab | Cara Mengatasi |
|---|---|---|
| `RESULT: {"error":"SOP held (a run is already in flight)"}` | Ada run lama yang menggantung di DB | Jalankan pembersihan DB di Tahap 3, lalu pkill dan restart daemon. |
| `error: required argument <RUN_ID> not provided` | Variabel `$RUN_ID` kosong karena daemon belum siap saat `curl` dipanggil | Pastikan daemon running (`curl http://127.0.0.1:42617/health`), lalu ulangi Tahap 4.1. |
| `cd: no such file or directory: plugins/...` | Berada di folder `zeroclaw` alih-alih `zeroclaw-solana-plugins` | Jalankan `cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins` terlebih dahulu. |
