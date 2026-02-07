# frozen_string_literal: true

module Monty
  class Run
    # Create a new Monty::Run instance by parsing Python code.
    #
    # @param code [String] Python source code
    # @param script_name [String] filename for error messages (default: "script.py")
    # @param inputs [Array<String>] input variable names (default: [])
    # @param external_functions [Array<String>] external function names (default: [])
    # @return [Monty::Run]
    #
    # @example Simple function
    #   run = Monty::Run.new("def add(x, y): return x + y", inputs: ["x", "y"])
    #
    # @example With external functions
    #   run = Monty::Run.new(code, external_functions: ["fetch"])
    #
    def self.new(code, script_name: "script.py", inputs: [], external_functions: [])
      _new(code, script_name, inputs, external_functions)
    end

    # Deserialize a Run from bytes previously created by #dump.
    #
    # @param bytes [String] serialized bytes
    # @return [Monty::Run]
    def self.load(bytes)
      _load(bytes)
    end

    # Execute the Python code with the given inputs.
    #
    # @param inputs positional arguments matching the input variable names
    # @param limits [Hash, nil] resource limits (max_allocations:, max_duration:, max_memory:, etc.)
    # @param capture_output [Boolean] if true, returns a Hash with :result and :output keys
    # @return [Object] the Python return value converted to Ruby, or Hash if capture_output
    #
    # @example Simple call
    #   run = Monty::Run.new("def add(x, y): return x + y", inputs: ["x", "y"])
    #   run.call(1, 2) # => 3
    #
    # @example With limits
    #   run.call(1, 2, limits: { max_duration: 5.0, max_memory: 1_048_576 })
    #
    # @example Capturing output
    #   run = Monty::Run.new("print('hello')\nresult = 42", inputs: [])
    #   run.call(capture_output: true) # => { result: 42, output: "hello\n" }
    #
    def call(*inputs, limits: nil, capture_output: false)
      input_array = inputs

      if capture_output
        if limits
          _run_capturing_with_limits(input_array, limits)
        else
          _run_capturing(input_array)
        end
      elsif limits
        _run_with_limits(input_array, limits)
      else
        _run(input_array)
      end
    end

    # Start iterative execution for scripts with external function calls.
    #
    # Returns a Monty::FunctionCall, Monty::PendingFutures, or Monty::Complete
    # depending on where execution paused.
    #
    # NOTE: This consumes the Run. It cannot be used again after calling start.
    #
    # @param inputs positional arguments matching the input variable names
    # @return [Monty::FunctionCall, Monty::PendingFutures, Monty::Complete]
    #
    # @example
    #   run = Monty::Run.new(code, external_functions: ["fetch"])
    #   progress = run.start
    #
    #   while progress.is_a?(Monty::FunctionCall)
    #     result = handle_call(progress.function_name, progress.args)
    #     progress = progress.resume(result)
    #   end
    #
    #   final_value = progress.value  # Monty::Complete
    #
    def start(*inputs)
      _start(inputs)
    end

    # Execute with a block that handles external function calls.
    #
    # The block receives a FunctionCall object and should return the result.
    # Execution continues automatically until completion.
    #
    # @param inputs positional arguments matching the input variable names
    # @yield [Monty::FunctionCall] called when Python invokes an external function
    # @yieldreturn [Object] the return value to provide to the Python code
    # @return [Object] the final Python return value converted to Ruby
    #
    # @example
    #   result = run.call_with_externals(input) do |call|
    #     case call.function_name
    #     when "fetch"
    #       http_get(call.args[0])
    #     else
    #       raise "Unknown function: #{call.function_name}"
    #     end
    #   end
    #
    def call_with_externals(*inputs, &block)
      raise ArgumentError, "a block is required" unless block_given?

      progress = start(*inputs)

      loop do
        case progress
        when Monty::Complete
          return progress.value
        when Monty::FunctionCall
          result = yield progress
          progress = progress.resume(result)
        when Monty::PendingFutures
          raise Monty::Error, "async futures are not supported by call_with_externals"
        else
          raise Monty::Error, "unexpected progress type: #{progress.class}"
        end
      end
    end

    # Serialize this Run to bytes for later restoration via Run.load
    #
    # @return [String] serialized bytes
    def dump
      _dump
    end
  end
end
