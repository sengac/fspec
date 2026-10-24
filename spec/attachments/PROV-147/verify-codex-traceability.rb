require 'json'

root = File.expand_path('../../..', __dir__)
Dir.chdir(root)
features = Dir['spec/features/*.feature'].select do |path|
  File.read(path).match?(/@PROV-1(?:4[7-9]|5[0-8])\b/)
end.sort
errors = []
seen = {}
count = 0

features.each do |feature|
  specifications = File.read(feature).split(/^  Scenario: /).drop(1).map do |block|
    steps = block.lines.drop(1).filter_map do |line|
      line.strip if line.match?(/^\s+(Given|When|Then|And|But) /)
    end
    [block.lines.first.strip, steps]
  end.to_h
  coverage = JSON.parse(File.read(feature + '.coverage')).fetch('scenarios')
  unless specifications.keys.sort == coverage.map { |scenario| scenario.fetch('name') }.sort
    errors << "#{feature}: scenario names differ"
  end
  coverage.each do |scenario|
    count += 1
    name = scenario.fetch('name')
    mappings = scenario.fetch('testMappings')
    errors << "#{name}: expected one mapping" unless mappings.length == 1
    mappings.each do |mapping|
      first, last = mapping.fetch('lines').split('-').map(&:to_i)
      lines = File.readlines(mapping.fetch('file'))
      body = lines[(first - 1)..(last - 1)].join
      functions = body.scan(/(?:async )?fn (\w+)\(/).flatten
      unless functions.length == 1 && body.match?(/#\[(?:tokio::)?test(?:\([^\n]*\))?\]/)
        errors << "#{name}: expected one annotated test function"
      end
      key = [mapping.fetch('file'), functions.first]
      errors << "#{name}: duplicate test function" if seen[key]
      seen[key] = true
      errors << "#{name}: invalid test range" unless first > 0 && last <= lines.length
      specifications.fetch(name).each do |step|
        errors << "#{name}: missing #{step}" unless body.include?("@step #{step}")
      end
      implementations = mapping.fetch('implMappings')
      errors << "#{name}: no implementation mapping" if implementations.empty?
      implementations.each do |implementation|
        length = File.readlines(implementation.fetch('file')).length
        unless implementation.fetch('lines').all? { |number| number > 0 && number <= length }
          errors << "#{name}: invalid implementation range in #{implementation.fetch('file')}"
        end
      end
    end
  end
end

puts "Features: #{features.length}; scenarios: #{count}; unique test functions: #{seen.length}; errors: #{errors.length}"
puts errors
exit(errors.empty? ? 0 : 1)
