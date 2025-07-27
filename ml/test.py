logs = [
    "User login successful",
    "User logout successful",
    "File uploaded",
    "Database connection established",
    "Scheduled task executed",
    "Error: disk full",
    "Warning: high memory usage",
    "Login failed: invalid credentials",
    "Memory allocation failure",
    "Kernel panic"
]

from sklearn.feature_extraction.text import TfidfVectorizer


normal_logs = logs[:5]  # Train only on normal logs
vectorizer = TfidfVectorizer(max_features=100)  # Convert text to vectors
X_train = vectorizer.fit_transform(normal_logs).toarray()

import torch
import torch.nn as nn

class LogAutoencoder(nn.Module):
    def __init__(self, input_dim):
        super().__init__()
        self.encoder = nn.Sequential(
            nn.Linear(input_dim, 16),
            nn.ReLU(),
            nn.Linear(16, 4)
        )
        self.decoder = nn.Sequential(
            nn.Linear(4, 16),
            nn.ReLU(),
            nn.Linear(16, input_dim)
        )

    def forward(self, x):
        encoded = self.encoder(x)
        decoded = self.decoder(encoded)
        return decoded

from torch.utils.data import DataLoader, TensorDataset

X_tensor = torch.tensor(X_train, dtype=torch.float32)
dataset = DataLoader(TensorDataset(X_tensor), batch_size=2, shuffle=True)

model = LogAutoencoder(input_dim=X_train.shape[1])
criterion = nn.MSELoss()  # Mean Squared Error = reconstruction loss
optimizer = torch.optim.Adam(model.parameters(), lr=0.01)

for epoch in range(50):
    total_loss = 0
    for batch in dataset:
        x = batch[0]
        output = model(x)
        loss = criterion(output, x)

        optimizer.zero_grad()
        loss.backward()
        optimizer.step()

        total_loss += loss.item()
    print(f"Epoch {epoch+1}: Loss = {total_loss:.4f}")


from torch.utils.data import DataLoader, TensorDataset

X_tensor = torch.tensor(X_train, dtype=torch.float32)
dataset = DataLoader(TensorDataset(X_tensor), batch_size=2, shuffle=True)

model = LogAutoencoder(input_dim=X_train.shape[1])
criterion = nn.MSELoss()  # Mean Squared Error = reconstruction loss
optimizer = torch.optim.Adam(model.parameters(), lr=0.01)

for epoch in range(50):
    total_loss = 0
    for batch in dataset:
        x = batch[0]
        output = model(x)
        loss = criterion(output, x)

        optimizer.zero_grad()
        loss.backward()
        optimizer.step()

        total_loss += loss.item()
    print(f"Epoch {epoch+1}: Loss = {total_loss:.4f}")


X_all = vectorizer.transform(logs).toarray()
X_all_tensor = torch.tensor(X_all, dtype=torch.float32)

with torch.no_grad():
    recon = model(X_all_tensor)
    errors = ((X_all_tensor - recon) ** 2).mean(dim=1)

# Set anomaly threshold
threshold = errors[:5].mean() + 3 * errors[:5].std()

for i, error in enumerate(errors):
    label = "Anomaly" if error > threshold else "Normal"
    print(f"{label}: {logs[i]} (Error: {error:.4f})")


