# Scripts

This directory owns deterministic project automations.

Rules:

- use scripts for stable, repeatable command workflows
- prefer scripts over repeated prose once a manual sequence is repeated enough
- keep scripts usable without paid services by default
- keep script inputs explicit and shell-friendly
- let CI call the same scripts whenever that keeps local and remote verification aligned
- keep one script per verification lane when that avoids hidden CI-only behavior
- keep network-dependent checks advisory until they prove cheap and stable enough for the default path
- let `infra-smoke.sh` own standalone real-infra checks when a full compose stack is unnecessary
- let `check-git-discipline.sh` enforce that completed waves are verified from a clean committed tree
- let `check-performance-baseline.sh` own local benchmarkable hot-path baselines before larger optimization waves change behavior
