// Test file for multiline patterns
function singleLine() { return "hello"; }

function multiLine() {
    const x = 10;
    const y = 20;
    return x + y;
}

class TestClass {
    constructor(name) {
        this.name = name;
        this.value = 0;
    }

    async fetchData() {
        try {
            const response = await fetch('/api/data');
            return response.json();
        } catch (error) {
            console.log('Error:', error);
            throw error;
        }
    }
}

if (condition) {
    console.log("true branch");
} else {
    console.log("false branch");
}

try {
    riskyOperation();
} catch (err) {
    handleError(err);
} finally {
    cleanup();
}