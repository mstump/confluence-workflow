ARG PYTHON_VERSION=3.11-slim
FROM python:${PYTHON_VERSION}

# Set environment variables
ENV PYTHONDONTWRITEBYTECODE=1
ENV PYTHONUNBUFFERED=1
ENV PLANTUML_JAR_PATH=/usr/share/plantuml/plantuml.jar
ENV VIRTUAL_ENV=/app/.venv
ENV PATH="${VIRTUAL_ENV}/bin:${JAVA_HOME}/bin:${PATH}"
ENV PUPPETEER_SKIP_CHROMIUM_DOWNLOAD=true
ENV PUPPETEER_EXECUTABLE_PATH=/usr/bin/chromium

# Install system dependencies including Java, PlantUML, Node.js, and Chromium
RUN apt-get update && apt-get install -y --no-install-recommends \
    openjdk-21-jre \
    plantuml \
    wget \
    nodejs \
    npm \
    chromium \
    fonts-liberation \
    && rm -rf /var/lib/apt/lists/*

# Install mermaid-cli
RUN npm install -g @mermaid-js/mermaid-cli

# Set work directory
WORKDIR /app

# Install uv
RUN pip install uv

# Copy dependencies and install
COPY pyproject.toml uv.lock ./
RUN uv sync

# Copy project
COPY src/ /app/src/

# Install the project
RUN uv pip install .

# Set the entrypoint
ENTRYPOINT ["confluence-agent"]
