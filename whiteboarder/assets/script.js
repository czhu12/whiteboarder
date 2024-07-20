// script.js
const canvas = document.getElementById('whiteboard');
const ctx = canvas.getContext('2d');
const clearButton = document.getElementById('clear');
const brushColorPicker = document.getElementById('brushColorPicker');
const eraserColorPicker = document.getElementById('eraserColorPicker');
const sizePicker = document.getElementById('sizePicker');
const eraserButton = document.getElementById('eraser');
const penButton = document.getElementById('pen');
const penSideBar = document.getElementById('pen-sidebar');
const undoButton = document.getElementById('undo');
const redoButton = document.getElementById('redo');

let painting = false;
let selectedButton = 'pen';
let strokes = [];
let redoStack = [];
let currentStroke = null;
let boardId;

const path = window.location.pathname;
const match = path.match(/^\/boards\/(.+)$/);

if (match) {
    boardId = match[1]; // Extract the ID
    console.log(boardId);
} else {
    console.log("Creating a new board...")
}

canvas.width = window.innerWidth;
canvas.height = window.innerHeight;

function startPosition(e) {
    painting = true;
    currentStroke = {
        color: brushColorPicker.value,
        size: sizePicker.value,
        points: []
    };
    draw(e);
}

function undo() {
    if (strokes.length === 0) return;
    const stroke = strokes.pop();
    redoStack.push(stroke);
    redraw();
}

function redo() {
    if (redoStack.length === 0) return;
    const stroke = redoStack.pop();
    strokes.push(stroke);
    redraw();
}

function endPosition() {
    painting = false;
    ctx.beginPath();
    if (currentStroke) {
        strokes.push(currentStroke);
        currentStroke = null;
        save(); // Call the save function after each mouse up event
    }
}

function draw(e) {
    if (!painting) return;

    const x = e.clientX;
    const y = e.clientY;

    currentStroke.points.push({ x, y });

    ctx.lineWidth = currentStroke.size;
    ctx.lineCap = 'round';
    ctx.strokeStyle = currentStroke.color;

    ctx.lineTo(x, y);
    ctx.stroke();
    ctx.beginPath();
    ctx.moveTo(x, y);
}

function redraw() {
    drawGuidelines();
    for (const stroke of strokes) {
        ctx.beginPath();
        ctx.lineWidth = stroke.size;
        ctx.lineCap = 'round';
        ctx.strokeStyle = stroke.color;
        for (const point of stroke.points) {
            ctx.lineTo(point.x, point.y);
            ctx.stroke();
            ctx.beginPath();
            ctx.moveTo(point.x, point.y);
        }
    }
    ctx.beginPath();
}

function eraseStroke(e) {
    const x = e.clientX;
    const y = e.clientY;
    const tolerance = sizePicker.value * 3; // Increase the tolerance area

    strokes = strokes.filter(stroke => {
        return !stroke.points.some(point => {
            const dx = point.x - x;
            const dy = point.y - y;
            return Math.sqrt(dx * dx + dy * dy) <= tolerance;
        });
    });

    redraw();
    save(); // Call the save function after erasing a stroke
}

canvas.addEventListener('mousedown', (e) => {
    if (selectedButton === 'eraser') {
        eraseStroke(e);
    } else {
        startPosition(e);
    }
});
canvas.addEventListener('mouseup', endPosition);
canvas.addEventListener('mousemove', (e) => {
    if (selectedButton === 'eraser') {
        eraseStroke(e);
    } else {
        draw(e);
    }
});

clearButton.addEventListener('click', () => {
    drawGuidelines();
    strokes = [];
    save(); // Call the save function after clearing the canvas
});

penButton.addEventListener('click', () => {
    selectedButton = 'pen';
    penButton.classList.add('active');
    eraserButton.classList.remove('active');
    penSideBar.classList.toggle('hidden');
})

eraserButton.addEventListener('click', () => {
    selectedButton = 'eraser';
    eraserButton.classList.add('active');
    penButton.classList.remove('active');
});

window.addEventListener('resize', () => {
    const imageData = ctx.getImageData(0, 0, canvas.width, canvas.height);
    canvas.width = window.innerWidth;
    canvas.height = window.innerHeight;
    drawGuidelines();
    ctx.putImageData(imageData, 0, 0);
});

undoButton.addEventListener('click', () => {
    undo();
});

redoButton.addEventListener('click', () => {
    redo();
});

document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.key === 'z') {
        undo();
        e.preventDefault();
    } else if (e.ctrlKey && e.key === 'r') {
        redo();
        e.preventDefault();
    }
});



// TODO: Implement the save function
function save() {
    // This function will handle saving the canvas state
    
}

// Function to draw the guidelines
function drawGuidelines() {
    ctx.clearRect(0, 0, canvas.width, canvas.height);
    ctx.rect(0, 0, canvas.width, canvas.height);

    ctx.fillStyle = "#f2f2f2";
    ctx.fill();

    const step = 60; // Distance between guidelines
    const width = canvas.width;
    const height = canvas.height;

    ctx.strokeStyle = '#cccccc';
    ctx.lineWidth = 0.5;

    // Draw vertical lines
    for (let x = step; x < width; x += step) {
        ctx.beginPath();
        ctx.moveTo(x, 0);
        ctx.lineTo(x, height);
        ctx.stroke();
    }

    // Draw horizontal lines
    for (let y = step; y < height; y += step) {
        ctx.beginPath();
        ctx.moveTo(0, y);
        ctx.lineTo(width, y);
        ctx.stroke();
    }
    ctx.beginPath();
}

drawGuidelines();
