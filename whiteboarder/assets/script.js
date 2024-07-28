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
const showModalButton = document.querySelector('#help button')

let painting = false;
let selectedButton = 'pen';
let board = {
    id: null,
    strokes: [],
}
let redoStack = [];
let currentStroke = null;

const path = window.location.pathname;
const match = path.match(/^\/boards\/(.+)$/);

async function loadBoard(boardId) {
    const response = await fetch(`/api/boards/${boardId}`, {method: 'GET', headers: {'Content-Type': 'application/json'}});
    const data = await response.json();
    return data
}

async function createBoard() {
    const response = await fetch("/api/boards", {method: 'POST', headers: {'Content-Type': 'application/json'}});
    const data = await response.json();
    return data
}

async function saveBoard() {
    const response = await fetch(`/api/boards/${board.id}`, {method: 'PUT', headers: {'Content-Type': 'application/json'}, body: JSON.stringify(board)});
    return response.status;
}

async function initializeBoard() {
    // This is where we should show a loading bar...
    if (match) {
        board = await loadBoard(match[1]);
    } else {
        board = await createBoard();
        window.history.replaceState(`board/${board.id}`, `Board ${board.id}`, `/boards/${board.id}`);
    }

    const location = window.location;
    const svgUrl = "https://" + location.hostname + location.pathname + ".svg"
    document.getElementById("example-url").value = svgUrl;
    console.log(board)
    redraw();
}
initializeBoard();

canvas.width = window.innerWidth;
canvas.height = window.innerHeight;

function startPosition(e) {
    painting = true;
    currentStroke = {
        color: brushColorPicker.value,
        size: parseInt(sizePicker.value),
        points: []
    };
    draw(e);
}

function undo() {
    if (board.strokes.length === 0) return;
    const stroke = board.strokes.pop();
    redoStack.push(stroke);
    redraw();
    saveBoard();
}

function redo() {
    if (redoStack.length === 0) return;
    const stroke = redoStack.pop();
    board.strokes.push(stroke);
    redraw();
    saveBoard();
}

function endPosition() {
    painting = false;
    ctx.beginPath();
    if (currentStroke) {
        board.strokes.push(currentStroke);
        currentStroke = null;
        saveBoard(); // Call the save function after each mouse up event
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
    for (const stroke of board.strokes) {
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

    board.strokes = board.strokes.filter(stroke => {
        return !stroke.points.some(point => {
            const dx = point.x - x;
            const dy = point.y - y;
            return Math.sqrt(dx * dx + dy * dy) <= tolerance;
        });
    });

    redraw();
    saveBoard(); // Call the save function after erasing a stroke
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
    board.strokes = [];
    saveBoard(); // Call the save function after clearing the canvas
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
    redraw();
    //drawGuidelines();
    //ctx.putImageData(imageData, 0, 0);
});

undoButton.addEventListener('click', () => {
    undo();
});

redoButton.addEventListener('click', () => {
    redo();
});
showModalButton.addEventListener('click', () => {
    // Show modal
})

document.addEventListener('keydown', (e) => {
    if (e.ctrlKey && e.key === 'z') {
        undo();
        e.preventDefault();
    } else if (e.ctrlKey && e.key === 'r') {
        redo();
        e.preventDefault();
    }
});

document.querySelector('.colorPicker .color').addEventListener('click', (e) => {
    console.log(e);
})

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
