package io.github.norbertque.pixeltone.ui.library

import android.app.Application
import androidx.lifecycle.AndroidViewModel
import androidx.lifecycle.viewModelScope
import io.github.norbertque.pixeltone.PixeltoneApp
import io.github.norbertque.pixeltone.library.AlbumSort
import io.github.norbertque.pixeltone.library.LibraryIndex
import io.github.norbertque.pixeltone.library.TrackSort
import io.github.norbertque.pixeltone.library.sortAlbums
import io.github.norbertque.pixeltone.library.sortTracks
import io.github.norbertque.pixeltone.library.SearchResult
import kotlinx.coroutines.Dispatchers
import kotlinx.coroutines.flow.MutableStateFlow
import kotlinx.coroutines.flow.SharingStarted
import kotlinx.coroutines.flow.StateFlow
import kotlinx.coroutines.flow.asStateFlow
import kotlinx.coroutines.flow.combine
import kotlinx.coroutines.flow.flowOn
import kotlinx.coroutines.flow.stateIn

class LibraryViewModel(app: Application) : AndroidViewModel(app) {
    private val library = (app as PixeltoneApp).library
    private val settings = (app as PixeltoneApp).librarySettings
    private val searchQuery = MutableStateFlow("")

    val index: StateFlow<LibraryIndex?> = library.index
    val query: StateFlow<String> = searchQuery.asStateFlow()
    val trackSort: StateFlow<TrackSort> = settings.trackSort
    val albumSort: StateFlow<AlbumSort> = settings.albumSort

    /** The library narrowed to the search query, its tracks and albums in the chosen order; null until it has been read. */
    val results: StateFlow<SearchResult?> =
        combine(library.index, searchQuery, settings.trackSort, settings.albumSort) { index, query, trackSort, albumSort ->
            index?.search(query)?.let { it.copy(tracks = sortTracks(it.tracks, trackSort), albums = sortAlbums(it.albums, albumSort)) }
        }
            .flowOn(Dispatchers.Default)
            .stateIn(viewModelScope, SharingStarted.WhileSubscribed(5_000), null)

    fun setTrackSort(sort: TrackSort) = settings.setTrackSort(sort)

    fun setAlbumSort(sort: AlbumSort) = settings.setAlbumSort(sort)

    fun search(query: String) {
        searchQuery.value = query
    }
}
