# frozen_string_literal: true

RSpec.describe Monty::Run do
  describe ".new" do
    it "parses valid Python code" do
      run = Monty::Run.new("x + 1", inputs: ["x"])
      expect(run).to be_a(Monty::Run)
    end

    it "returns the source code" do
      code = "x + 1"
      run = Monty::Run.new(code, inputs: ["x"])
      expect(run.code).to eq(code)
    end

    it "raises SyntaxError for invalid Python" do
      expect { Monty::Run.new("def !!!") }.to raise_error(Monty::SyntaxError)
    end

    it "uses default script_name" do
      run = Monty::Run.new("42")
      expect(run).to be_a(Monty::Run)
    end
  end

  describe "#call" do
    it "evaluates a simple expression" do
      run = Monty::Run.new("x + y", inputs: ["x", "y"])
      expect(run.call(1, 2)).to eq(3)
    end

    it "returns None as nil" do
      run = Monty::Run.new("None")
      expect(run.call).to be_nil
    end

    it "returns booleans" do
      run = Monty::Run.new("True")
      expect(run.call).to eq(true)

      run = Monty::Run.new("False")
      expect(run.call).to eq(false)
    end

    it "returns integers" do
      run = Monty::Run.new("42")
      expect(run.call).to eq(42)
    end

    it "returns floats" do
      run = Monty::Run.new("3.14")
      expect(run.call).to be_within(0.001).of(3.14)
    end

    it "returns strings" do
      run = Monty::Run.new("'hello world'")
      expect(run.call).to eq("hello world")
    end

    it "returns lists as arrays" do
      run = Monty::Run.new("[1, 2, 3]")
      expect(run.call).to eq([1, 2, 3])
    end

    it "returns dicts as hashes" do
      run = Monty::Run.new("{'a': 1, 'b': 2}")
      expect(run.call).to eq({ "a" => 1, "b" => 2 })
    end

    it "returns tuples as frozen arrays" do
      run = Monty::Run.new("(1, 2, 3)")
      result = run.call
      expect(result).to eq([1, 2, 3])
      expect(result).to be_frozen
    end

    it "handles nested data structures" do
      run = Monty::Run.new("{'items': [1, 2, 3], 'nested': {'a': True}}")
      result = run.call
      expect(result["items"]).to eq([1, 2, 3])
      expect(result["nested"]).to eq({ "a" => true })
    end

    it "passes Ruby values as inputs" do
      run = Monty::Run.new("name.upper()", inputs: ["name"])
      expect(run.call("hello")).to eq("HELLO")
    end

    it "passes arrays as lists" do
      run = Monty::Run.new("len(items)", inputs: ["items"])
      expect(run.call([1, 2, 3])).to eq(3)
    end

    it "passes hashes as dicts" do
      run = Monty::Run.new("data['key']", inputs: ["data"])
      expect(run.call({ "key" => 42 })).to eq(42)
    end

    it "can be called multiple times" do
      run = Monty::Run.new("x * 2", inputs: ["x"])
      expect(run.call(5)).to eq(10)
      expect(run.call(10)).to eq(20)
      expect(run.call(0)).to eq(0)
    end

    it "handles functions" do
      code = <<~PYTHON
        def factorial(n):
            if n <= 1:
                return 1
            return n * factorial(n - 1)

        factorial(n)
      PYTHON

      run = Monty::Run.new(code, inputs: ["n"])
      expect(run.call(5)).to eq(120)
      expect(run.call(10)).to eq(3_628_800)
    end
  end

  describe "#call with capture_output" do
    it "captures print output" do
      code = <<~PYTHON
        print('hello')
        print('world')
        42
      PYTHON

      run = Monty::Run.new(code)
      result = run.call(capture_output: true)

      expect(result[:result]).to eq(42)
      expect(result[:output]).to include("hello")
      expect(result[:output]).to include("world")
    end
  end

  describe "#call with limits" do
    it "accepts resource limit options" do
      run = Monty::Run.new("x + 1", inputs: ["x"])
      result = run.call(1, limits: { max_duration: 5.0 })
      expect(result).to eq(2)
    end
  end

  describe "#start" do
    it "pauses at external function calls" do
      code = <<~PYTHON
        result = fetch("https://example.com")
        result
      PYTHON

      run = Monty::Run.new(code, external_functions: ["fetch"])
      progress = run.start

      expect(progress).to be_a(Monty::FunctionCall)
      expect(progress.function_name).to eq("fetch")
      expect(progress.args).to eq(["https://example.com"])
    end

    it "resumes execution with provided result" do
      code = <<~PYTHON
        result = fetch("https://example.com")
        result
      PYTHON

      run = Monty::Run.new(code, external_functions: ["fetch"])
      progress = run.start

      expect(progress).to be_a(Monty::FunctionCall)
      progress = progress.resume("response data")

      expect(progress).to be_a(Monty::Complete)
      expect(progress.value).to eq("response data")
    end

    it "handles multiple external calls" do
      code = <<~PYTHON
        a = fetch("url1")
        b = fetch("url2")
        a + ' ' + b
      PYTHON

      run = Monty::Run.new(code, external_functions: ["fetch"])
      progress = run.start

      expect(progress).to be_a(Monty::FunctionCall)
      expect(progress.function_name).to eq("fetch")
      progress = progress.resume("first")

      expect(progress).to be_a(Monty::FunctionCall)
      expect(progress.function_name).to eq("fetch")
      progress = progress.resume("second")

      expect(progress).to be_a(Monty::Complete)
      expect(progress.value).to eq("first second")
    end
  end

  describe "#call_with_externals" do
    it "handles external function calls via block" do
      code = <<~PYTHON
        result = fetch("https://example.com")
        result.upper()
      PYTHON

      run = Monty::Run.new(code, external_functions: ["fetch"])
      result = run.call_with_externals do |call|
        case call.function_name
        when "fetch" then "hello from ruby"
        end
      end

      expect(result).to eq("HELLO FROM RUBY")
    end

    it "raises without a block" do
      run = Monty::Run.new("42")
      expect { run.call_with_externals }.to raise_error(ArgumentError)
    end
  end

  describe "#dump / .load" do
    it "round-trips serialization" do
      run = Monty::Run.new("x + 1", inputs: ["x"])
      bytes = run.dump

      restored = Monty::Run.load(bytes)
      expect(restored.call(41)).to eq(42)
    end
  end

  describe "error handling" do
    it "raises Monty::Error for Python runtime errors" do
      run = Monty::Run.new("1 / 0")
      expect { run.call }.to raise_error(Monty::Error)
    end

    it "raises Monty::SyntaxError for syntax errors" do
      expect { Monty::Run.new("def !!!") }.to raise_error(Monty::SyntaxError)
    end

    it "provides error hierarchy" do
      expect(Monty::SyntaxError).to be < Monty::Error
      expect(Monty::ResourceError).to be < Monty::Error
      expect(Monty::ConsumedError).to be < Monty::Error
      expect(Monty::Error).to be < StandardError
    end
  end
end
