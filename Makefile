.PHONY: validate status ready preflight

# Secondary convenience targets for macOS/Linux. Windows is canonical; use scripts/*.ps1 there.
validate:
	node scripts/validate-bootstrap.mjs

status:
	node scripts/taskctl.mjs status

ready:
	node scripts/taskctl.mjs ready

preflight:
	./scripts/preflight.sh
