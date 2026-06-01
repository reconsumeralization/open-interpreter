import os
import stat
import subprocess
import tempfile
import textwrap
import unittest
from pathlib import Path
from shlex import quote


REPO_ROOT = Path(__file__).resolve().parents[2]
INSTALL_SH = REPO_ROOT / "scripts" / "install" / "install.sh"


class InstallShLatestResolutionTests(unittest.TestCase):
    def run_installer_with_release_list(self, release_list: str) -> subprocess.CompletedProcess[str]:
        with tempfile.TemporaryDirectory() as tmpdir:
            tmp = Path(tmpdir)
            release_list_path = tmp / "releases.json"
            release_list_path.write_text(release_list, encoding="utf-8")
            fake_bin = tmp / "bin"
            fake_bin.mkdir()
            fake_curl = fake_bin / "curl"
            fake_curl.write_text(
                textwrap.dedent(
                    f"""\
                    #!/bin/sh
                    url="${{@:$#}}"
                    case "$url" in
                      */releases?per_page=100)
                        cat {quote(str(release_list_path))}
                        ;;
                      */releases/tags/rust-v0.2.0)
                        printf '{{"assets":[]}}\\n'
                        ;;
                      *)
                        echo "unexpected curl URL: $url" >&2
                        exit 42
                        ;;
                    esac
                    """
                ),
                encoding="utf-8",
            )
            fake_curl.chmod(fake_curl.stat().st_mode | stat.S_IXUSR)

            env = {
                **os.environ,
                "PATH": f"{fake_bin}{os.pathsep}{os.environ['PATH']}",
                "CODEX_GITHUB_REPO": "KillianLucas/oix",
                "CODEX_INSTALL_PRODUCT_NAME": "Open Interpreter",
                "CODEX_PACKAGE_ASSET_STEM": "open-interpreter-package",
                "CODEX_COMMAND_NAME": "interpreter",
                "CODEX_RELEASE_TAG_PREFIX": "rust-v",
                "CODEX_NON_INTERACTIVE": "1",
                "CODEX_HOME": str(tmp / "home"),
                "CODEX_INSTALL_DIR": str(tmp / "install-bin"),
            }
            return subprocess.run(
                [str(INSTALL_SH)],
                cwd=REPO_ROOT,
                env=env,
                text=True,
                stdout=subprocess.PIPE,
                stderr=subprocess.PIPE,
                check=False,
            )

    def test_latest_without_matching_tag_prefix_fails_at_version_resolution(self) -> None:
        result = self.run_installer_with_release_list(
            textwrap.dedent(
                """\
                [
                  {
                    "tag_name": "v0.0.8",
                    "draft": false,
                    "prerelease": false
                  }
                ]
                """
            )
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn(
            "Failed to resolve the latest Open Interpreter release version.",
            result.stderr,
        )

    def test_latest_uses_matching_non_prerelease_tag_prefix(self) -> None:
        result = self.run_installer_with_release_list(
            textwrap.dedent(
                """\
                [
                  {
                    "tag_name": "v0.0.8",
                    "draft": false,
                    "prerelease": false
                  },
                  {
                    "tag_name": "rust-v0.3.0-beta.1",
                    "draft": false,
                    "prerelease": true
                  },
                  {
                    "tag_name": "rust-v0.2.0",
                    "draft": false,
                    "prerelease": false
                  }
                ]
                """
            )
        )

        self.assertNotEqual(result.returncode, 0)
        self.assertIn("Could not find Open Interpreter release assets for 0.2.0.", result.stderr)


if __name__ == "__main__":
    unittest.main()
