# frozen_string_literal: true

require_relative "lib/monty/version"

Gem::Specification.new do |spec|
  spec.name = "monty-rb"
  spec.version = Monty::VERSION
  spec.authors = ["Monty Contributors"]
  spec.email = []

  spec.summary = "Ruby bindings for Monty (monty-rb)"
  spec.description = "A minimal, secure Python interpreter for AI agents — Ruby bindings via Magnus"
  spec.homepage = "https://github.com/catkins/monty-rb"
  spec.license = "MIT"
  spec.required_ruby_version = ">= 3.2.0"

  spec.metadata["homepage_uri"] = spec.homepage
  spec.metadata["source_code_uri"] = "https://github.com/catkins/monty-rb"
  spec.metadata["changelog_uri"] = "https://github.com/catkins/monty-rb/blob/main/CHANGELOG.md"
  spec.metadata["rubygems_mfa_required"] = "true"

  spec.files = Dir[
    "lib/**/*.rb",
    "ext/**/*.{rb,rs,toml}",
    "Cargo.toml",
    "LICENSE",
    "README.md"
  ]

  spec.bindir = "bin"
  spec.require_paths = ["lib"]
  spec.extensions = ["ext/monty/extconf.rb"]

  spec.add_dependency "rb_sys", "~> 0.9"

  spec.add_development_dependency "rake", "~> 13.0"
  spec.add_development_dependency "rake-compiler", "~> 1.2"
  spec.add_development_dependency "rspec", "~> 3.12"
end
