# frozen_string_literal: true

RSpec.describe "MontyObject conversion" do
  describe "Ruby -> Python -> Ruby round-trip" do
    it "converts nil" do
      run = Monty::Run.new("x", inputs: ["x"])
      expect(run.call(nil)).to be_nil
    end

    it "converts true" do
      run = Monty::Run.new("x", inputs: ["x"])
      expect(run.call(true)).to eq(true)
    end

    it "converts false" do
      run = Monty::Run.new("x", inputs: ["x"])
      expect(run.call(false)).to eq(false)
    end

    it "converts integers" do
      run = Monty::Run.new("x", inputs: ["x"])
      expect(run.call(0)).to eq(0)
      expect(run.call(42)).to eq(42)
      expect(run.call(-100)).to eq(-100)
    end

    it "converts floats" do
      run = Monty::Run.new("x", inputs: ["x"])
      expect(run.call(3.14)).to be_within(0.001).of(3.14)
      expect(run.call(0.0)).to eq(0.0)
      expect(run.call(-1.5)).to eq(-1.5)
    end

    it "converts strings" do
      run = Monty::Run.new("x", inputs: ["x"])
      expect(run.call("hello")).to eq("hello")
      expect(run.call("")).to eq("")
      expect(run.call("unicode: \u00e9\u00e0\u00fc")).to eq("unicode: \u00e9\u00e0\u00fc")
    end

    it "converts arrays to lists" do
      run = Monty::Run.new("x", inputs: ["x"])
      expect(run.call([])).to eq([])
      expect(run.call([1, 2, 3])).to eq([1, 2, 3])
      expect(run.call(["a", "b"])).to eq(["a", "b"])
    end

    it "converts hashes to dicts" do
      run = Monty::Run.new("x", inputs: ["x"])
      expect(run.call({})).to eq({})
      expect(run.call({ "key" => "value" })).to eq({ "key" => "value" })
    end

    it "converts nested structures" do
      run = Monty::Run.new("x", inputs: ["x"])
      input = { "list" => [1, 2, 3], "nested" => { "a" => true } }
      expect(run.call(input)).to eq(input)
    end

    it "converts symbols to strings" do
      run = Monty::Run.new("x", inputs: ["x"])
      expect(run.call(:hello)).to eq("hello")
    end
  end

  describe "Python -> Ruby type mapping" do
    it "maps None to nil" do
      run = Monty::Run.new("None")
      expect(run.call).to be_nil
    end

    it "maps True/False to true/false" do
      expect(Monty::Run.new("True").call).to eq(true)
      expect(Monty::Run.new("False").call).to eq(false)
    end

    it "maps int to Integer" do
      run = Monty::Run.new("42")
      expect(run.call).to be_a(Integer)
    end

    it "maps float to Float" do
      run = Monty::Run.new("3.14")
      expect(run.call).to be_a(Float)
    end

    it "maps str to String" do
      run = Monty::Run.new("'hello'")
      expect(run.call).to be_a(String)
    end

    it "maps list to Array" do
      run = Monty::Run.new("[1, 2, 3]")
      expect(run.call).to be_a(Array)
    end

    it "maps dict to Hash" do
      run = Monty::Run.new("{'a': 1}")
      expect(run.call).to be_a(Hash)
    end

    it "maps tuple to frozen Array" do
      run = Monty::Run.new("(1, 2)")
      result = run.call
      expect(result).to be_a(Array)
      expect(result).to be_frozen
    end
  end

  describe "Python operations on Ruby inputs" do
    it "string operations" do
      run = Monty::Run.new("s.upper()", inputs: ["s"])
      expect(run.call("hello")).to eq("HELLO")
    end

    it "list operations" do
      run = Monty::Run.new("len(items)", inputs: ["items"])
      expect(run.call([1, 2, 3])).to eq(3)
    end

    it "dict operations" do
      run = Monty::Run.new("list(d.keys())", inputs: ["d"])
      expect(run.call({ "a" => 1, "b" => 2 })).to contain_exactly("a", "b")
    end

    it "arithmetic on inputs" do
      run = Monty::Run.new("x ** 2 + y", inputs: ["x", "y"])
      expect(run.call(3, 1)).to eq(10)
    end
  end
end
