"""Failure-path checks for the actual LightCraft inventory wrapper."""
import importlib.util
import io
import os
from pathlib import Path
import subprocess
import sys
import tempfile
import unittest
from unittest.mock import patch

sys.dont_write_bytecode = True
SOURCE = Path(__file__).resolve().with_name('generate-cargo-sources.py')
spec = importlib.util.spec_from_file_location('inventory_wrapper', SOURCE)
wrapper = importlib.util.module_from_spec(spec)
spec.loader.exec_module(wrapper)


class InventoryFailures(unittest.TestCase):
    def test_download_rejects_changed_bytes(self):
        with patch.object(wrapper, 'urlopen', return_value=io.BytesIO(b'changed upstream')):
            with self.assertRaisesRegex(SystemExit, 'checksum mismatch'):
                wrapper.download_upstream()

    def test_download_rejects_excessive_size(self):
        with patch.object(wrapper, 'urlopen', return_value=io.BytesIO(b'x' * 1_000_001)):
            with self.assertRaisesRegex(SystemExit, 'unexpectedly large'):
                wrapper.download_upstream()

    def test_patch_refuses_unrecognized_source(self):
        with self.assertRaisesRegex(SystemExit, 'matched 0 times'):
            wrapper.patched_upstream(b'not the expected generator')

    def test_cache_cannot_be_inside_checkout_or_target(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            for cache in (root / 'repo', root / 'repo/subdir', root / 'target', root / 'target/subdir'):
                with patch.dict(os.environ, {'CARGO_TARGET_DIR': str(root / 'target')}):
                    with self.assertRaises(SystemExit):
                        wrapper.ensure_outside_source(cache, root / 'repo')

    def test_relative_target_uses_generator_working_directory(self):
        with tempfile.TemporaryDirectory() as temp:
            root = Path(temp).resolve()
            # The wrapper can be invoked elsewhere; its child runs in repo_root.
            elsewhere = root / 'elsewhere/deeper'
            elsewhere.mkdir(parents=True)
            previous = Path.cwd()
            try:
                os.chdir(elsewhere)
                with patch.dict(os.environ, {'CARGO_TARGET_DIR': '../target'}):
                    with self.assertRaisesRegex(SystemExit, 'outside CARGO_TARGET_DIR'):
                        wrapper.ensure_outside_source(root / 'target/cache', root / 'repo')
            finally:
                os.chdir(previous)

    def test_failed_or_invalid_generation_preserves_inventory(self):
        for code, payload in [(17, b'partial'), (0, b''), (0, b'invalid json'), (0, b'[]'), (0, b'{}')]:
            with self.subTest(code=code, payload=payload), tempfile.TemporaryDirectory() as temp:
                root = Path(temp).resolve()
                lock = root / 'Cargo.lock'
                lock.write_text('version = 4\n')
                output = root / 'sources.json'
                output.write_bytes(b'original inventory')
                def generate(args, **kwargs):
                    Path(args[-1]).write_bytes(payload)
                    return subprocess.CompletedProcess(args, code)
                args = ['generate', str(lock), '-o', str(output), '--cache-dir', str(root / 'cache')]
                with patch.object(sys, 'argv', args), patch.object(wrapper, 'download_upstream', return_value=b'fixture'), patch.object(wrapper, 'patched_upstream', return_value=b'fixture'), patch.object(wrapper.subprocess, 'run', side_effect=generate):
                    if code:
                        self.assertEqual(wrapper.main(), code)
                    else:
                        with self.assertRaises((SystemExit, ValueError)):
                            wrapper.main()
                self.assertEqual(output.read_bytes(), b'original inventory')
                self.assertEqual(list(root.glob('.sources.json.*.tmp')), [])


if __name__ == '__main__':
    unittest.main(verbosity=2)
