# frozen_string_literal: true

require "bundler/gem_tasks"
require "rspec/core/rake_task"
require "rb_sys/extensiontask"

RSpec::Core::RakeTask.new(:spec)

GEMSPEC = Gem::Specification.load("monty-rb.gemspec")

RbSys::ExtensionTask.new("monty", GEMSPEC) do |ext|
  ext.lib_dir = "lib/monty"
  ext.ext_dir = "ext/monty"
end

task default: %i[compile spec]
task test: :spec
