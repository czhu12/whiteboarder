function convertToSnakeCase(text) {
    // Convert the text to lowercase
    let lowerCased = text.toLowerCase();
    // Replace spaces with underscores
    let snakeCased = lowerCased.replace(/\s+/g, '_');
    return snakeCased;
}

function throttleAndDebounce(func, wait) {
  let timeout;
  let lastExecution = 0;

  return function(...args) {
    const context = this;
    const now = Date.now();

    const executeFunction = () => {
      lastExecution = now;
      func.apply(context, args);
    };

    clearTimeout(timeout);

    if (now - lastExecution >= wait) {
      executeFunction();
    } else {
      timeout = setTimeout(executeFunction, wait);
    }
  };
}

// script.js
const canvas = document.getElementById('whiteboard');
const ctx = canvas.getContext('2d');
const clearButton = document.getElementById('clear');
const brushColorPicker = document.getElementById('brushColorPicker');
const sizePicker = document.getElementById('sizePicker');
const eraserButton = document.getElementById('eraser');
const penButton = document.getElementById('pen');
const penSideBar = document.getElementById('pen-sidebar');
penSideBar.style.left = `${penButton.getBoundingClientRect().x + 70}px`;
penSideBar.style.top = `${penButton.getBoundingClientRect().y}px`;
const undoButton = document.getElementById('undo');
const redoButton = document.getElementById('redo');
const showModalButton = document.querySelector('#help i');
const collaborators = {};

let hasMouseDown = false;
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
    redraw();
}
initializeBoard();

canvas.width = window.innerWidth;
canvas.height = window.innerHeight;

function startPosition(e) {
    currentStroke = {
        color: brushColorPicker.value,
        size: parseInt(sizePicker.value),
        points: [],
        timestamp: Date.now()
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
    ctx.beginPath();
    if (currentStroke) {
        board.strokes.push(currentStroke);
        currentStroke = null;
        saveBoard(); // Call the save function after each mouse up event
    }
}

function draw(e) {
    if (!hasMouseDown) return;

    const x = e.clientX;
    const y = e.clientY;

    currentStroke.points.push({ x, y });
    redraw();
}

function redraw() {
    drawGuidelines();
    for (const stroke of board.strokes.concat(currentStroke).filter(e => !!e)) {
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

function deleteCursor(username) {
    const elementId = convertToSnakeCase(username);
    const element = document.getElementById(elementId);
    element.remove();
}

function drawCursors() {
    // Set the position of the cursor
    for (const [collaboraterUsername, cursor] of Object.entries(collaborators)) {
        const elementId = convertToSnakeCase(collaboraterUsername);
        if (!document.getElementById(elementId)) {
            const parent = document.getElementById('cursors');
            const element = `
                <div class="cursor" id="${elementId}">
                    <i class="bi bi-pen-fill"></i>
                    ${collaboraterUsername}
                </div>
            `
            parent.insertAdjacentHTML('beforeend', element)
        }
        const {x, y} = cursor;
        const element = document.getElementById(elementId)
        element.style.left = `${x+10}px`;
        element.style.top = `${y+10}px`;
    }
}

function eraseStroke(e) {
    if (!hasMouseDown) return;
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
    hasMouseDown = true;
    if (selectedButton === 'eraser') {
        eraseStroke(e);
    } else {
        startPosition(e);
    }
});

canvas.addEventListener('mouseup', (e) => {
    hasMouseDown = false;
    endPosition(e)
});

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

document.querySelectorAll('.colorPicker .color').forEach(el => {
    el.addEventListener('click', (e) => {
        brushColorPicker.value = e.target.dataset.value
    })
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

// Websockets

// Arrays of adjectives and animals
const adjectives = ['Happy', 'Brave', 'Clever', 'Swift', 'Calm', 'Fierce', 'Gentle', 'Wise', 'Bold', 'Sly'];
const animals = ['Lion', 'Tiger', 'Bear', 'Wolf', 'Fox', 'Eagle', 'Hawk', 'Shark', 'Panther', 'Falcon'];

// Function to generate a random name
function generateRandomName() {
    const randomAdjective = adjectives[Math.floor(Math.random() * adjectives.length)];
    const randomAnimal = animals[Math.floor(Math.random() * animals.length)];
    return `${randomAdjective} ${randomAnimal} ${Math.floor(Math.random() * 1000)}`;
}

// Generate and log a random name
let socket;
const username = generateRandomName();

function connect(boardId) {
    const origin = window.location.origin;
    const wsUrl = `${origin.replace(/^http/, 'ws')}/ws`;
    socket = new WebSocket(wsUrl);

    socket.addEventListener("open", () => {
        console.log(`Connecting as ${username}`)
        const data = JSON.stringify({username, channel: `boards/${boardId}`})
        socket.send(data);
    });

    socket.addEventListener('message', function (event) {
        const data = JSON.parse(event.data);
        if (data.messagetype === "cursor") {
            if (data.payload.username !== username) {
                collaborators[data.payload.username] = data.payload
                drawCursors();
            }
        } else if (data.messagetype === "board") {
            // Update board value
            board = data.payload.payload;
            redraw();
        } else if (data.messagetype === "userleft") {
            delete collaborators[data.payload.payload];
            deleteCursor(data.payload.payload);
        }
    })
}

const handleMouseMove = throttleAndDebounce(function (event) {
    if (socket) {
        const data = JSON.stringify({
            channel: `boards/${board.id}`,
            messagetype: "cursor",
            payload: { username, x: event.clientX, y: event.clientY }
        })
        socket.send(data);
    }
}, 50); // Adjust the wait time (in milliseconds) as needed

document.onmousemove = handleMouseMove;

if (match) {
    connect(match[1]);
}