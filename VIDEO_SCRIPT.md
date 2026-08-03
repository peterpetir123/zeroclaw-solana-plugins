# 🎬 Video Recording Script — Solana DeFi Guardian Bounty Demo
# Estimated Duration: ~3-4 minutes
# Jalankan perintah satu per satu, tunggu output sebelum lanjut.

---

## SCENE 1: Intro & Unit Tests (~30 detik)

```bash
# Masuk ke direktori repo plugin
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins

# Jalankan 49 unit test (solana-lite: 29, token-risk-check: 11, spl-transfer-build: 9)
(cd plugins/solana-lite && cargo test)
(cd plugins/token-risk-check && cargo test)
(cd plugins/spl-transfer-build && cargo test)
```

---

## SCENE 2: Live Mainnet Risk Audit — CLI (~30 detik)

```bash
# Audit USDC token langsung di mainnet Solana (tanpa perlu daemon)
(cd plugins/token-risk-check && cargo run --bin token-risk-check-cli EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v)
```

---

## SCENE 3: Start Daemon & Trigger SOP (~30 detik)

```bash
# Pastikan daemon ZeroClaw berjalan
curl -s http://127.0.0.1:42617/health | python3 -c "import sys,json; d=json.load(sys.stdin); print('Daemon:', d['status'], '| Uptime:', d['runtime']['uptime_seconds'], 'seconds')"

# Trigger SOP: solana-transfer-guard
RESULT=$(curl -s -X POST http://127.0.0.1:42617/api/sops/solana-transfer-guard/run \
  -H "Content-Type: application/json" \
  -d '{"payload": "{\"mint_address\": \"So11111111111111111111111111111111111111112\", \"from\": \"4zMMC9srt5Ri5X14GAgXhaHii3GnPAEERYPJgZJDncDU\", \"to\": \"675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8\", \"amount\": \"100000\"}"}')
echo "Trigger Result: $RESULT"

# Extract RUN_ID
export RUN_ID=$(echo $RESULT | python3 -c "import sys,json; print(json.load(sys.stdin).get('run_id',''))")
echo "RUN_ID: $RUN_ID"
```

---

## SCENE 4: SOP Step 1 → Approve → Step 2 (~30 detik)

```bash
# Cek overlay: Step 1 waiting_approval
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool

# Approve HITL Gate Step 1
zeroclaw sop approve $RUN_ID
```

---

## SCENE 5: Polling Step 2 → Step 3 → Approve Step 4 (~60 detik)

```bash
# Tunggu ~5 detik, lalu cek overlay lagi
sleep 5
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool

# Kalau masih running, tunggu lagi
sleep 5
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool

# Kalau sudah waiting_approval di step 2, approve:
zeroclaw sop approve $RUN_ID

# Tunggu Step 3 (Build Tx) selesai dan Step 4 muncul
sleep 10
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool

# Tunggu sampai step 4 waiting_approval, lalu approve:
sleep 5
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool

# Approve final HITL Gate Step 4
zeroclaw sop approve $RUN_ID
```

---

## SCENE 6: Verifikasi Final — SOP Completed 4/4 (~20 detik)

```bash
# Tunggu selesai
sleep 5

# Cek overlay final: harus status=completed, semua step=completed
curl -s http://127.0.0.1:42617/api/sops/solana-transfer-guard/runs/$RUN_ID/overlay | python3 -m json.tool
```

> **Expected output:**
> ```json
> {
>     "status": "completed",
>     "current_step": 4,
>     "total_steps": 4,
>     "nodes": [
>         {"step": 1, "state": "completed"},
>         {"step": 2, "state": "completed"},
>         {"step": 3, "state": "completed"},
>         {"step": 4, "state": "completed"}
>     ]
> }
> ```

---

## SCENE 7: Prompt Injection Defense (~30 detik)

```bash
# Jalankan unit test prompt injection fail-closed
(cd plugins/spl-transfer-build && cargo test injection_via_amount_field_fails_closed -- --nocapture)
(cd plugins/spl-transfer-build && cargo test injection_via_recipient_with_extra_instructions -- --nocapture)
(cd plugins/spl-transfer-build && cargo test injection_via_memo_does_not_alter_recipient -- --nocapture)
(cd plugins/token-risk-check && cargo test prompt_injection_in_mint_address_fails_closed -- --nocapture)
```

---

## OPTIONAL — Kalau SOP Stuck / "SOP held"

```bash
# Clear stale claims
python3 -c "
import sqlite3
conn = sqlite3.connect('$HOME/.zeroclaw/data/sop/runs.db')
c = conn.cursor()
c.execute('DELETE FROM sop_claims')
c.execute('UPDATE sop_runs SET terminal = 1 WHERE terminal = 0')
conn.commit()
print('Stale claims cleared.')
"

# Restart daemon
pkill zeroclaw; sleep 2
cd /home/hengkerprotzy/coding/zeroclaw && ./target/release/zeroclaw daemon -v &
sleep 3
cd /home/hengkerprotzy/coding/zeroclaw-solana-plugins
```
