let currentUploadAppId = null;

document.addEventListener("DOMContentLoaded", () => {
    loadGames();

    const searchBar = document.querySelector('.search-bar');
    if (searchBar) {
        searchBar.addEventListener('input', (e) => {
            loadGames(e.target.value);
        });
    }
});

async function loadGames(query = "") {
    const grid = document.getElementById("games-grid");
    if (!grid) return;

    try {
        const response = await fetch(`http://127.0.0.1:3000/api/games?q=${encodeURIComponent(query)}`);
        const games = await response.json();
        renderGames(games);
    } catch (err) {
        console.error("failed to load:", err);
    }
}

function renderGames(games) {
    const grid = document.getElementById("games-grid");
    grid.innerHTML = "";

    games.forEach(game => {
        const card = document.createElement("div");
        card.className = "game-card";

        const hasValidCover = game.cover_url && game.cover_url.trim() !== "";

card.innerHTML = `
            <div class="card-actions">
                <button class="action-icon-btn" title="Rename" onclick="renameGame('${game.appid}', '${game.name}')">
                    <img src="../1img/pen-symbol.png" alt="Rename" class="btn-icon">
                </button>
                ${game.is_custom ? `
                <button class="action-icon-btn delete" title="Delete" onclick="deleteGame('${game.appid}')">
                    <img src="../1img/basket-symbol.png" alt="Delete" class="btn-icon">
                </button>` : ''}
            </div>
            
            <div class="game-title" title="${game.name}">${game.name}</div>
            
            <div class="game-cover-container">
                ${hasValidCover ? 
                    `<img class="game-cover" src="${game.cover_url}" alt="${game.name}" onerror="handleImageError(this, '${game.appid}')">
                    <div class="cover-overlay">
                        <button class="action-icon-btn" title="Change Image" onclick="triggerImageUpload('${game.appid}')">
                            <img src="../1img/painting-symbol.png" alt="Change Image" class="btn-icon">
                        </button>
                        <button class="action-icon-btn" title="Remove Image" onclick="removeImage('${game.appid}')">
                            <img src="../1img/cross-symbol.png" alt="Remove Image" class="btn-icon">
                        </button>
                        <button class="action-icon-btn" title="Revert Image" onclick="revertImage('${game.appid}')">
                            <img src="../1img/revert-icon.png" alt="Revert Image" class="btn-icon">
                        </button>   
                    </div>` : 
                    `<button class="no-image-btn" onclick="triggerImageUpload('${game.appid}')">+</button>`
                }
            </div>
            
            <button class="play-btn" onclick="launchGame('${game.appid}')">
                <div class="play-icon"></div>
            </button>
        `;
        grid.appendChild(card);
    });
}

function handleImageError(imgElement, appid) {
    const container = imgElement.parentElement;
    container.innerHTML = `<button class="no-image-btn" onclick="triggerImageUpload('${appid}')">+</button>`;
}

function triggerImageUpload(appid) {
    currentUploadAppId = appid;
    document.getElementById("cover-upload-input").click();
}

async function handleImageUpload(event) {
    const file = event.target.files[0];
    if (!file || !currentUploadAppId) return;

    const formData = new FormData();
    formData.append("appid", currentUploadAppId);
    formData.append("file", file);

    await fetch("http://127.0.0.1:3000/api/upload-image", { method: "POST", body: formData });
    loadGames();
}

async function removeImage(appid) {
    await fetch("http://127.0.0.1:3000/api/remove-image", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ appid })
    });
    loadGames();
}

async function renameGame(appid, oldName) {
    const newName = prompt("New Name:", oldName);
    if (!newName || newName === oldName) return;

    await fetch("http://127.0.0.1:3000/api/rename-game", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ appid, name: newName })
    });
    loadGames();
}

async function deleteGame(appid) {
    if (!confirm("Are you sure?")) return;

    await fetch("http://127.0.0.1:3000/api/delete-game", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ appid })
    });
    loadGames();
}

async function launchGame(appid) {
    fetch(`http://127.0.0.1:3000/api/launch?appid=${appid}`);
}

function openAddGameModal() {
    document.getElementById("add-game-modal").style.display = "flex";
}

function closeAddGameModal() {
    document.getElementById("add-game-modal").style.display = "none";
}

async function saveCustomGame() {
    const name = document.getElementById("custom-title").value;
    const launch_path = document.getElementById("custom-path").value;
    const cover_url = document.getElementById("custom-cover").value;

    if (!name || !launch_path) {
        alert("At least one title and path.");
        return;
    }

    await fetch("http://127.0.0.1:3000/api/add-game", {
        method: "POST",
        headers: { "Content-Type": "application/json" },
        body: JSON.stringify({ 
            name, 
            launch_path, 
            cover_url,
            category: "riot" // <-- HIER DIE JEWEILIGE KATEGORIE EINTRAGEN ("steam", "epic", "riot", "windows", etc.)
        })
    });
    closeAddGameModal();
    loadGames();
}
async function loadGames(query = "") {
    const grid = document.getElementById("games-grid");
    if (!grid) return;

    try {
        // EXAKT DIESE ZEILE ANPASSEN:
        const response = await fetch(`http://127.0.0.1:3000/api/riot-games?q=${encodeURIComponent(query)}`);
        const games = await response.json();
        renderGames(games);
    } catch (err) {
        console.error("failed to load:", err);
    }
}