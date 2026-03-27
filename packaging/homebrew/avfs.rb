class Avfs < Formula
  desc "Virtual filesystem CLI backed by embedded databases for AI agents"
  homepage "https://github.com/neul-labs/agentvfs"
  url "https://github.com/neul-labs/agentvfs/archive/refs/tags/v0.1.0.tar.gz"
  sha256 "PLACEHOLDER_SHA256"
  license "MIT"
  head "https://github.com/neul-labs/agentvfs.git", branch: "main"

  depends_on "rust" => :build

  def install
    system "cargo", "install", *std_cargo_args

    # Install man page
    man1.install "man/avfs.1"

    # Generate shell completions
    generate_completions_from_executable(bin/"avfs", "completions")
  end

  test do
    # Create a test vault
    system "#{bin}/avfs", "vault", "create", "test-vault"

    # Write a file
    system "#{bin}/avfs", "write", "/test.txt", "Hello, Homebrew!"

    # Read it back and verify
    output = shell_output("#{bin}/avfs cat /test.txt")
    assert_match "Hello, Homebrew!", output

    # Clean up
    system "#{bin}/avfs", "vault", "delete", "test-vault"
  end
end
