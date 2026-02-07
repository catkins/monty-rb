# frozen_string_literal: true

require_relative "monty/version"

# Load the native extension
begin
  RUBY_VERSION =~ /(\d+\.\d+)/
  require "monty/#{Regexp.last_match(1)}/monty"
rescue LoadError
  require "monty/monty"
end

# Load Ruby class extensions
require_relative "monty/run"
